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

use std::{collections::HashMap, sync::Arc};

use sc_client_api::backend::Finalizer;
use sc_consensus::LongestChain;
use sp_consensus::{
    import_queue::CacheKeyId, BlockCheckParams, BlockImport, BlockImportParams, ImportResult,
};
use sp_runtime::{generic::BlockId, Justification};

use substrate_test_runtime_client::runtime::Block;

/// Full client for test network
pub type FullClient = sc_service::client::Client<
    substrate_test_runtime_client::Backend,
    substrate_test_runtime_client::Executor,
    Block,
    substrate_test_runtime_client::runtime::RuntimeApi,
>;

/// Client chain
pub type ClientChain = LongestChain<substrate_test_runtime_client::Backend, Block>;

/// Mock network client
#[derive(Clone)]
pub struct Client {
    pub(crate) inner: Arc<FullClient>,
    pub(crate) backend: Arc<substrate_test_runtime_client::Backend>,
    pub(crate) chain: ClientChain,
}

impl Client {
    /// Implementation for [`sc_client_api::backend::Finalizer`]
    pub fn finalize_block(
        &self,
        id: BlockId<Block>,
        justification: Option<Justification>,
        notify: bool,
    ) -> sp_blockchain::Result<()> {
        self.inner.finalize_block(id, justification, notify)
    }

    // Return a clone of the client longest chain
    pub fn chain(&self) -> ClientChain {
        self.chain.clone()
    }
}

#[async_trait::async_trait]
impl BlockImport<Block> for Client {
    type Error = sp_consensus::Error;

    type Transaction = ();

    /// Check block preconditions
    async fn check_block(
        &mut self,
        block: BlockCheckParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner.check_block(block).await
    }

    /// Import a block
    async fn import_block(
        &mut self,
        block: BlockImportParams<Block, Self::Transaction>,
        cache: HashMap<CacheKeyId, Vec<u8>>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner
            .import_block(block.clear_storage_changes_and_mutate(), cache)
            .await
    }
}
