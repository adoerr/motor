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

use std::sync::Arc;

use {
    sc_network::config::ProtocolConfig,
    sc_service::client::Client,
    sp_consensus::{
        block_import::BlockImport,
        import_queue::{BoxJustificationImport, Verifier},
    },
};

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
    client: Arc<FullClient>,
    backend: Arc<substrate_test_runtime_client::Backend>,
}

pub trait MockNetwork {
    type Verifier: Verifier<MockBlock> + 'static;

    type BlockImport: BlockImport<MockBlock, Error = sp_consensus::Error>
        + Clone
        + Send
        + Sync
        + 'static;

    type PeerData: Default;

    /// Implement this method to return a mock network customized for your needs.
    fn new() -> Self;

    /// Implement this method to return a block import verifier customized for your needs.
    fn verifier(
        &self,
        client: MockClient,
        config: &ProtocolConfig,
        data: &Self::PeerData,
    ) -> Self::Verifier;

    /// Implement this method to return a block import implementation customized for your needs.
    fn block_import(
        &self,
        client: MockClient,
    ) -> (
        Self::BlockImport,
        Option<BoxJustificationImport<MockBlock>>,
        Self::PeerData,
    );
}
