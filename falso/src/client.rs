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
use sc_service::client::Client;
use sp_consensus::{
    import_queue::CacheKeyId, BlockCheckParams, BlockImport, BlockImportParams, ImportResult,
};
use sp_runtime::{generic::BlockId, Justification};

use substrate_test_runtime_client::runtime::Block as MockBlock;

/// Full client for test network
pub type FullClient = Client<
    substrate_test_runtime_client::Backend,
    substrate_test_runtime_client::Executor,
    MockBlock,
    substrate_test_runtime_client::runtime::RuntimeApi,
>;

/// Mock network client
#[derive(Clone)]
pub struct MockClient {
    inner: Arc<FullClient>,
    backend: Arc<substrate_test_runtime_client::Backend>,
}

impl MockClient {
    /// Implementation for [`sc_client_api::backend::Finalizer`]
    pub fn finalize_block(
        &self,
        id: BlockId<MockBlock>,
        justification: Option<Justification>,
        notify: bool,
    ) -> sp_blockchain::Result<()> {
        self.inner.finalize_block(id, justification, notify)
    }
}

#[async_trait::async_trait]
impl BlockImport<MockBlock> for MockClient {
    type Error = sp_consensus::Error;

    type Transaction = ();

    /// Check block preconditions
    async fn check_block(
        &mut self,
        block: BlockCheckParams<MockBlock>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner.check_block(block).await
    }

    /// Import a block
    async fn import_block(
        &mut self,
        block: BlockImportParams<MockBlock, Self::Transaction>,
        cache: HashMap<CacheKeyId, Vec<u8>>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner
            .import_block(block.convert_transaction(), cache)
            .await
    }
}
