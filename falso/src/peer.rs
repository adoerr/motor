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

use std::{borrow::Cow, pin::Pin, sync::Arc};

use sc_client_api::{client::BlockImportNotification, FinalityNotification};
use sc_consensus::LongestChain;
use sc_network::{Multiaddr, NetworkWorker};
use sp_consensus::{import_queue::Verifier, BlockImport};
use substrate_test_runtime_client::{
    runtime::{Block, Hash},
    Backend,
};

use futures::{lock::Mutex as AsyncMutex, Stream};

use crate::Client;

type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

#[derive(Default)]
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
    pub(crate) verfifier: Arc<AsyncMutex<Box<dyn Verifier<Block>>>>,
    pub(crate) block_import: BI,
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
    fn new() -> Self {
        todo!()
    }
}
