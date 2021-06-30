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

use sc_network::config::ProtocolConfig;
use sp_consensus::{
    block_import::BlockImport,
    import_queue::{BoxJustificationImport, Verifier},
    ForkChoiceStrategy,
};

use substrate_test_runtime_client::runtime::Block;

use crate::{import::PassThroughVerifier, Client, Peer};

pub trait NetworkProvider {
    type Verifier: Verifier<Block> + 'static;

    type BlockImport: BlockImport<Block, Error = sp_consensus::Error>
        + Clone
        + Send
        + Sync
        + 'static;

    type Link: Default;

    /// Implement this method to return a mock network customized for your needs.
    fn new() -> Self;

    /// Implement this method to return a block import verifier customized for your needs.
    fn verifier(
        &self,
        client: Client,
        config: &ProtocolConfig,
        link: &Self::Link,
    ) -> Self::Verifier;

    /// Implement this method to return a block import implementation customized for your needs.
    fn block_import(
        &self,
        client: Client,
    ) -> (
        Self::BlockImport,
        Option<BoxJustificationImport<Block>>,
        Self::Link,
    );
}

pub struct Network {
    #[allow(dead_code)]
    peers: Vec<Peer<Client>>,
    #[allow(dead_code)]
    fork_choice: ForkChoiceStrategy,
}

impl NetworkProvider for Network {
    type Verifier = PassThroughVerifier;
    type BlockImport = Client;
    type Link = ();

    fn new() -> Self {
        Network {
            peers: Vec::new(),
            fork_choice: ForkChoiceStrategy::LongestChain,
        }
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
        Self::BlockImport,
        Option<BoxJustificationImport<Block>>,
        Self::Link,
    ) {
        (client, None, ())
    }
}
