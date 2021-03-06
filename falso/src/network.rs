// Copyright (C) 2021 Andreas Doerr
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{
    sync::Arc,
    task::{Context, Poll},
};

use sc_client_api::BlockchainEvents;
use sc_client_db::{
    Backend, DatabaseSettings, DatabaseSettingsSrc, KeepBlocks, PruningMode, TransactionStorageMode,
};
use sc_network::{
    block_request_handler::BlockRequestHandler,
    config::{
        build_multiaddr, EmptyTransactionPool, NetworkConfiguration, NonDefaultSetConfig,
        ProtocolConfig, ProtocolId, Role, SetConfig, SyncMode, TransportConfig,
    },
    light_client_requests::handler::LightClientRequestHandler,
    state_request_handler::StateRequestHandler,
    NetworkWorker,
};
use sp_consensus::{
    block_import::BlockImport,
    block_validation::DefaultBlockAnnounceValidator,
    import_queue::{BasicQueue, BoxJustificationImport, Verifier},
};

use substrate_test_runtime_client::{runtime::Block, TestClientBuilder, TestClientBuilderExt};

use futures::{prelude::*, FutureExt};
use futures_core::future::BoxFuture;
use log::trace;

use crate::{
    import::TrackingVerifier, AnyBlockImport, Client, Finalizer, PassThroughVerifier, Peer,
    PeerConfig,
};

pub trait NetworkProvider {
    type Verifier: Verifier<Block> + Clone + 'static;

    type BlockImport: BlockImport<Block, Error = sp_consensus::Error>
        + Clone
        + Send
        + Sync
        + 'static;

    type Link: Default;

    /// Implement this function to return a mock network customized for your needs.
    fn new() -> Self;

    /// Implement this function to return a block import verifier customized for your needs.
    fn verifier(
        &self,
        client: Client,
        config: &ProtocolConfig,
        link: &Self::Link,
    ) -> Self::Verifier;

    /// Implement this function to return a block import implementation customized for your needs.
    fn block_import(
        &self,
        client: Client,
    ) -> (
        AnyBlockImport<Self::BlockImport>,
        Option<BoxJustificationImport<Block>>,
        Self::Link,
    );

    /// Implment this function to return a mutable reference to peer `i`
    fn peer(&mut self, i: usize) -> &mut Peer<Self::Link, Self::BlockImport>;

    /// Implement this function to return a reference to the vector of peers
    fn peers(&self) -> &Vec<Peer<Self::Link, Self::BlockImport>>;

    /// Implement this function to mutate all peers with a `mutator`
    fn mutate_peers<M>(&mut self, mutator: M)
    where
        M: FnOnce(&mut Vec<Peer<Self::Link, Self::BlockImport>>);

    #[allow(dead_code)]
    /// Add a peer with `config` peer configuration
    fn add_peer(&mut self, config: PeerConfig) {
        let client = client();

        let (block_import, justification_import, link) = self.block_import(client.clone());

        let verifier = self.verifier(client.clone(), &Default::default(), &link);
        let verifier = TrackingVerifier::new(verifier);

        let import_queue = Box::new(BasicQueue::new(
            verifier.clone(),
            Box::new(block_import.clone()),
            justification_import,
            &sp_core::testing::TaskExecutor::new(),
            None,
        ));

        let protocol_id = ProtocolId::from("falso-protocol-name");

        let block_request_protocol_config = {
            let (handler, protocol_config) =
                BlockRequestHandler::new(&protocol_id, client.inner.clone(), 50);
            self.spawn_task(handler.run().boxed());
            protocol_config
        };

        let state_request_protocol_config = {
            let (handler, protocol_config) =
                StateRequestHandler::new(&protocol_id, client.inner.clone(), 50);
            self.spawn_task(handler.run().boxed());
            protocol_config
        };

        let light_client_request_protocol_config = {
            let (handler, protocol_config) =
                LightClientRequestHandler::new(&protocol_id, client.inner.clone());
            self.spawn_task(handler.run().boxed());
            protocol_config
        };

        let net_cfg = network_config(config.clone());

        let network = NetworkWorker::new(sc_network::config::Params {
            role: if config.is_authority {
                Role::Authority
            } else {
                Role::Full
            },
            executor: None,
            transactions_handler_executor: Box::new(|tsk| {
                async_std::task::spawn(tsk);
            }),
            network_config: net_cfg.clone(),
            chain: client.inner.clone(),
            on_demand: None,
            transaction_pool: Arc::new(EmptyTransactionPool),
            protocol_id,
            import_queue,
            block_announce_validator: Box::new(DefaultBlockAnnounceValidator),
            metrics_registry: None,
            block_request_protocol_config,
            state_request_protocol_config,
            light_client_request_protocol_config,
        })
        .unwrap();

        self.mutate_peers(move |peers| {
            for peer in peers.iter_mut() {
                peer.network.add_known_address(
                    *network.service().local_peer_id(),
                    net_cfg.listen_addresses[0].clone(),
                );
            }

            let block_import_stream = Box::pin(client.inner.import_notification_stream().fuse());

            let finality_notification_stream =
                Box::pin(client.inner.finality_notification_stream().fuse());

            peers.push(Peer {
                link,
                client: client.clone(),
                verifier,
                block_import,
                select_chain: Some(client.chain()),
                network,
                block_import_stream,
                finality_notification_stream,
                listen_addr: net_cfg.listen_addresses[0].clone(),
            });
        });
    }

    /// Spawn background tasks
    fn spawn_task(&self, f: BoxFuture<'static, ()>) {
        async_std::task::spawn(f);
    }

    /// Poll the network. Polling will process all pending events
    ///
    /// Note that we merge multiple pending finality notifications together and only
    /// act on the last one. This is the same behaviour as (indirectly) exhibited by
    /// [`sc_service::build_network()`]
    fn poll(&mut self, cx: &mut Context) {
        self.mutate_peers(|peers| {
            for (i, peer) in peers.iter_mut().enumerate() {
                trace!(target: "falso", "Polling peer {}: {}", i, peer.id());

                if let Poll::Ready(()) = peer.network.poll_unpin(cx) {
                    panic!("Network worker terminated unexpectedly")
                }

                trace!(target: "falso", "Done polling peer {}: {}", i, peer.id());

                // process pending block import notifications
                while let Poll::Ready(Some(imported)) =
                    peer.block_import_stream.as_mut().poll_next(cx)
                {
                    peer.network.service().announce_block(imported.hash, None);
                }

                // merge pending finality notifications, only process the last one
                let mut last = None;

                while let Poll::Ready(Some(finalized)) =
                    peer.finality_notification_stream.as_mut().poll_next(cx)
                {
                    last = Some(finalized);
                }

                if let Some(finalized) = last {
                    peer.network
                        .on_block_finalized(finalized.hash, finalized.header);
                }
            }
        });
    }

    /// Poll the network, until all peers are connected to each other.
    fn poll_connected(&mut self, cx: &mut Context) -> Poll<()> {
        self.poll(cx);

        let others = self.peers().len() - 1;

        if self.peers().iter().all(|p| p.connected_peers() == others) {
            return Poll::Ready(());
        }

        Poll::Pending
    }

    /// Poll the network until all peers have synced
    fn poll_synced(&mut self, cx: &mut Context) -> Poll<()> {
        self.poll(cx);

        // we keep polling until all peers agree on the best block
        let mut best = None;

        for peer in self.peers().iter() {
            if peer.is_syncing() || peer.network.num_queued_blocks() != 0 {
                return Poll::Pending;
            }

            if peer.network.num_sync_requests() != 0 {
                return Poll::Pending;
            }

            match (best, peer.client.info().best_hash) {
                (None, hash) => best = Some(hash),
                (Some(ref a), ref b) if a == b => {}
                (Some(_), _) => return Poll::Pending,
            }
        }

        Poll::Ready(())
    }

    /// Block until all peers are connected to each other
    fn block_until_connected(&mut self) {
        futures::executor::block_on(futures::future::poll_fn::<(), _>(|cx| {
            self.poll_connected(cx)
        }))
    }

    /// Block until all peers finished syncing
    fn block_until_synced(&mut self) {
        futures::executor::block_on(futures::future::poll_fn::<(), _>(|cx| self.poll_synced(cx)))
    }
}

// Return a mock network client for a new peer
fn client() -> Client {
    let db = kvdb_memorydb::create(12);
    let db = sp_database::as_database(db);

    let db_settings = DatabaseSettings {
        state_cache_size: 16777216,
        state_cache_child_ratio: Some((50, 100)),
        state_pruning: PruningMode::default(),
        source: DatabaseSettingsSrc::Custom(db),
        keep_blocks: KeepBlocks::All,
        transaction_storage: TransactionStorageMode::BlockBody,
    };

    let backend = Backend::new(db_settings, 0).expect("failed to create test backend");
    let backend = Arc::new(backend);

    let builder = TestClientBuilder::with_backend(backend);

    let backend = builder.backend();

    let (client, chain) = builder.build_with_longest_chain();
    let inner = Arc::new(client);

    Client {
        inner,
        backend,
        chain,
    }
}

// Return a network configuration for a new peer
fn network_config(config: PeerConfig) -> NetworkConfiguration {
    let mut net_cfg =
        NetworkConfiguration::new("falso-node", "falso-client", Default::default(), None);

    net_cfg.sync_mode = SyncMode::Full;
    net_cfg.transport = TransportConfig::MemoryOnly;
    net_cfg.listen_addresses = vec![build_multiaddr![Memory(rand::random::<u64>())]];
    net_cfg.allow_non_globals_in_dht = true;
    net_cfg.default_peers_set = SetConfig::default();
    net_cfg.extra_sets = config
        .protocols
        .into_iter()
        .map(|p| NonDefaultSetConfig {
            notifications_protocol: p,
            fallback_names: Vec::new(),
            max_notification_size: 1024 * 1024,
            set_config: Default::default(),
        })
        .collect();

    net_cfg
}

/// A simple default network
pub struct Network {
    peers: Vec<Peer<(), Client>>,
}

impl NetworkProvider for Network {
    type Verifier = PassThroughVerifier;
    type BlockImport = Client;
    type Link = ();

    fn new() -> Self {
        Network { peers: Vec::new() }
    }

    fn verifier(
        &self,
        _client: Client,
        _config: &ProtocolConfig,
        _link: &Self::Link,
    ) -> Self::Verifier {
        PassThroughVerifier::new(false)
    }

    fn block_import(
        &self,
        client: Client,
    ) -> (
        AnyBlockImport<Self::BlockImport>,
        Option<BoxJustificationImport<Block>>,
        Self::Link,
    ) {
        (
            client.as_block_import(),
            Some(Box::new(Finalizer(client))),
            Default::default(),
        )
    }

    fn peer(&mut self, i: usize) -> &mut Peer<Self::Link, Self::BlockImport> {
        &mut self.peers[i]
    }

    fn peers(&self) -> &Vec<Peer<Self::Link, Self::BlockImport>> {
        &self.peers
    }

    fn mutate_peers<M>(&mut self, mutator: M)
    where
        M: FnOnce(&mut Vec<Peer<Self::Link, Self::BlockImport>>),
    {
        mutator(&mut self.peers);
    }
}

#[cfg(test)]
mod tests {
    use super::{Network, NetworkProvider, PeerConfig};

    #[test]
    fn new_network() {
        let _ = env_logger::try_init();

        let mut net = Network::new();

        assert_eq!(net.peers.len(), 0);

        net.add_peer(PeerConfig::default());
        net.add_peer(PeerConfig::default());

        assert_eq!(net.peers.len(), 2);

        let id1 = net.peer(0).id();
        let id2 = net.peer(1).id();

        assert_ne!(id1, id2);
        assert_eq!(0, net.peer(0).connected_peers());
        assert_eq!(0, net.peer(1).connected_peers());
    }

    #[test]
    fn connect_all_peers() {
        let _ = env_logger::try_init();

        let mut net = Network::new();

        for _ in 0..5 {
            net.add_peer(PeerConfig::default());
        }

        assert!(net.peers().iter().all(|p| p.connected_peers() == 0));

        net.block_until_connected();

        let others = net.peers().len() - 1;

        assert!(net.peers().iter().all(|p| p.connected_peers() == others));
    }
}
