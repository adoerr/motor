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
pub struct Peer<BI> {
    pub(crate) client: Client,
    pub(crate) verifier: TrackingVerifier<Block>,
    pub(crate) block_import: AnyBlockImport<BI>,
    pub(crate) select_chain: Option<LongestChain<Backend, Block>>,
    pub(crate) network: NetworkWorker<Block, Hash>,
    pub(crate) block_import_stream: BoxStream<BlockImportNotification<Block>>,
    pub(crate) finality_notification_stream: BoxStream<FinalityNotification<Block>>,
    pub(crate) listen_addr: Multiaddr,
}

impl<BI> Peer<BI>
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

    /// Return the number of peers this peer is connected to
    pub fn connected_peers(&self) -> usize {
        self.network.num_connected_peers()
    }

    /// Add a new block.
    ///
    /// Adding a new block will push the block through the block import pipeline.
    pub fn add_block(&mut self) -> Hash {
        let best = self.client.info().best_hash;

        self.block_at(BlockId::Hash(best), BlockOrigin::File, |b| {
            b.build().unwrap().block
        })
    }

    fn block_at<F>(&mut self, at: BlockId<Block>, origin: BlockOrigin, mut builder: F) -> H256
    where
        F: FnMut(BlockBuilder<Block, FullClient, Backend>) -> Block,
    {
        let client = self.client.as_full();
        let parent = client.header(&at).unwrap().unwrap().hash();

        let block = client
            .new_block_at(&BlockId::Hash(parent), Default::default(), false)
            .unwrap();

        let block = builder(block);
        let hash = block.header.hash();

        trace!(target: "falso", "Block {} #{} parent: {}", hash, block.header.number, parent);

        let (block_import, cache) =
            futures::executor::block_on(self.verifier.verify(origin, block.header, None, None))
                .unwrap();

        let cache = if let Some(cache) = cache {
            cache.into_iter().collect()
        } else {
            Default::default()
        };

        futures::executor::block_on(self.block_import.import_block(block_import, cache))
            .expect("import block failed");

        self.network.service().announce_block(hash, None);

        self.network.new_best_block_imported(
            hash,
            *client
                .header(&BlockId::Hash(hash))
                .ok()
                .flatten()
                .unwrap()
                .number(),
        );

        hash
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::PeerConfig;
    use crate::network::{Network, NetworkProvider};

    use sp_core::H256;

    #[test]
    fn add_single_block() {
        let mut net = Network::new();

        net.add_peer(PeerConfig::default());

        let want =
            H256::from_str("0x2b999b3e9eb1ad3f086f7ef961a38c71a7636fb76d2ccce8216da3ec0b9c7e8d")
                .unwrap();

        let got = net.peer(0).add_block();

        assert_eq!(want, got);
    }
}
