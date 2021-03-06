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

#![allow(dead_code)]

use std::{borrow::Cow, pin::Pin};

use sc_block_builder::{BlockBuilder, BlockBuilderProvider};
use sc_client_api::{client::BlockImportNotification, FinalityNotification};
use sc_consensus::LongestChain;
use sc_network::{Multiaddr, NetworkWorker};
use sp_consensus::{import_queue::Verifier, BlockImport, BlockOrigin};
use sp_core::H256;
use sp_runtime::{generic::BlockId, traits::Header};

use substrate_test_runtime_client::{
    runtime::{Block, Hash},
    Backend,
};

use futures::Stream;
use libp2p::PeerId;
use log::trace;

use crate::{client::FullClient, import::TrackingVerifier, AnyBlockImport, Client};

type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

#[derive(Default, Clone)]
/// Configuration for a network peer
pub struct PeerConfig {
    /// Set of notification protocols a peer should participate in.
    pub protocols: Vec<Cow<'static, str>>,
    /// Is peer an authority or a regualr node
    pub is_authority: bool,
}

/// A network peer
pub struct Peer<L, BI> {
    pub(crate) link: L,
    pub(crate) client: Client,
    pub(crate) verifier: TrackingVerifier<Block>,
    pub(crate) block_import: AnyBlockImport<BI>,
    pub(crate) select_chain: Option<LongestChain<Backend, Block>>,
    pub(crate) network: NetworkWorker<Block, Hash>,
    pub(crate) block_import_stream: BoxStream<BlockImportNotification<Block>>,
    pub(crate) finality_notification_stream: BoxStream<FinalityNotification<Block>>,
    pub(crate) listen_addr: Multiaddr,
}

impl<L, BI> Peer<L, BI>
where
    BI: BlockImport<Block, Error = sp_consensus::Error> + Send + Sync,
    BI::Transaction: Send,
{
    /// Return unique peer id
    pub fn id(&self) -> PeerId {
        *self.network.service().local_peer_id()
    }

    /// Return a reference to the network, i.e. the peer's network worker
    pub fn network(&self) -> &NetworkWorker<Block, Hash> {
        &self.network
    }

    /// Return a reference to the peer's client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Return the number of peers this peer is connected to
    pub fn connected_peers(&self) -> usize {
        self.network.num_connected_peers()
    }

    /// Return whether peer is currently syncing
    pub fn is_syncing(&self) -> bool {
        self.network.service().is_major_syncing()
    }

    /// Add a new block at best block.
    ///
    /// Adding a new block will push the block through the block import pipeline.
    pub fn add_block(&mut self) -> Hash {
        let best = self.client.info().best_hash;

        self.blocks_at(BlockId::Hash(best), 1, BlockOrigin::File, |b| {
            b.build().unwrap().block
        })
    }

    /// Add `count` blocks at best block
    ///
    /// Adding blocks will push them through the block import pipeline.
    pub fn add_blocks(&mut self, count: usize) -> Hash {
        let best = self.client.info().best_hash;

        self.blocks_at(BlockId::Hash(best), count, BlockOrigin::File, |b| {
            b.build().unwrap().block
        })
    }

    fn blocks_at<F>(
        &mut self,
        at: BlockId<Block>,
        count: usize,
        origin: BlockOrigin,
        mut builder: F,
    ) -> H256
    where
        F: FnMut(BlockBuilder<Block, FullClient, Backend>) -> Block,
    {
        let client = self.client.as_full();
        let mut at = client.header(&at).unwrap().unwrap().hash();

        for _ in 0..count {
            let block = client
                .new_block_at(&BlockId::Hash(at), Default::default(), false)
                .expect("new_block_at() failed");

            let block = builder(block);
            let hash = block.header.hash();

            trace!(target: "falso", "Block {} #{} parent: {}", hash, block.header.number, at);

            let (block_import, cache) =
                futures::executor::block_on(self.verifier.verify(origin, block.header, None, None))
                    .expect("verify block failed");

            let cache = if let Some(cache) = cache {
                cache.into_iter().collect()
            } else {
                Default::default()
            };

            futures::executor::block_on(self.block_import.import_block(block_import, cache))
                .expect("import block failed");

            self.network.service().announce_block(hash, None);

            at = hash;
        }
        self.network.new_best_block_imported(
            at,
            *client
                .header(&BlockId::Hash(at))
                .ok()
                .flatten()
                .unwrap()
                .number(),
        );

        at
    }
}

#[cfg(test)]
mod tests {
    use super::PeerConfig;

    use crate::network::{Network, NetworkProvider};

    #[test]
    fn add_single_block() {
        let _ = env_logger::try_init();

        let mut net = Network::new();

        net.add_peer(PeerConfig::default());
        net.peer(0).add_block();

        let best = net.peer(0).client().info().best_number;

        assert_eq!(1, best);
    }

    #[test]
    fn add_multiple_blocks() {
        let _ = env_logger::try_init();

        let mut net = Network::new();

        net.add_peer(PeerConfig::default());

        let hash = net.peer(0).add_blocks(5);

        net.block_until_synced();

        let best = net.peer(0).client().info().best_hash;

        assert_eq!(hash, best);
    }
}
