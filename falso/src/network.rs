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

use emptor::{AnyBlockImport, Client, Finalizer, PassThroughVerifier, TrackingVerifier};
use futures::{prelude::*, FutureExt};
use futures_core::future::BoxFuture;
use sc_client_api::BlockchainEvents;
use sc_consensus::{
    block_import::BlockImport,
    import_queue::{BoxJustificationImport, Verifier},
};
use sc_network::{
    config::{
        build_multiaddr, NetworkConfiguration, NonDefaultSetConfig, ProtocolId, Role, SetConfig,
        SyncMode, TransportConfig,
    },
    request_responses::ProtocolConfig,
    NetworkWorker,
};
use substrate_test_runtime_client::runtime::Block;
use tokio::task;
use tracing::trace;

use crate::{Peer, PeerConfig};

#[cfg(test)]
#[path = "network_tests.rs"]
mod tests;

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
        client: Arc<Client>,
        config: &ProtocolConfig,
        link: &Self::Link,
    ) -> Self::Verifier;

    /// Implement this function to return a block import implementation customized for your needs.
    fn block_import(
        &self,
        client: Arc<Client>,
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

    /// Add a peer with `config` peer configuration
    fn add_peer(&mut self, config: PeerConfig) {
        let client = Arc::new(Client::new());

        let (block_import, justification_import, link) = self.block_import(client.clone());

        let config = ProtocolConfig {
            name: From::from("falso-protocol-name"),
            fallback_names: vec![],
            max_request_size: 0,
            max_response_size: 0,
            request_timeout: Default::default(),
            inbound_queue: Default::default(),
        };

        let verifier = self.verifier(client.clone(), &config, &link);
        let verifier = TrackingVerifier::new(verifier);

        let protocol_id = ProtocolId::from("falso-protocol-name");

        // new PeerConfig() sets is_authority to false
        let peer_config = PeerConfig {
            is_authority: config.is_authority,
            ..Default::default()
        };

        let net_cfg = network_config(peer_config);

        let network = NetworkWorker::new(sc_network::config::Params {
            role: if config.is_authority {
                Role::Authority
            } else {
                Role::Full
            },
            executor: None,
            network_config: net_cfg.clone(),
            protocol_id,
            genesis_hash: (),
            fork_id: None,
            metrics_registry: None,
            block_announce_config: NonDefaultSetConfig {},
            tx: (),
            inbound_queue: None,
        })
        .unwrap();

        self.mutate_peers(move |peers| {
            for peer in peers.iter_mut() {
                peer.network.add_known_address(
                    *network.service().local_peer_id(),
                    net_cfg.listen_addresses[0].clone(),
                );
            }

            let block_import_stream =
                Box::pin(client.as_inner().import_notification_stream().fuse());

            let finality_notification_stream =
                Box::pin(client.as_inner().finality_notification_stream().fuse());

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
        task::spawn(f);
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
        _client: Arc<Client>,
        _config: &ProtocolConfig,
        _link: &Self::Link,
    ) -> Self::Verifier {
        PassThroughVerifier::new(false)
    }

    fn block_import(
        &self,
        client: Arc<Client>,
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
