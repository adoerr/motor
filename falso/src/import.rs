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

use sp_consensus::{
    import_queue::{CacheKeyId, Verifier},
    BlockImportParams, BlockOrigin, ForkChoiceStrategy,
};
use sp_runtime::{traits::Block, Justifications};

/// A Verifier that accepts all justifications and passes them on for import.
///
/// Block finality and fork choice strategy are configurable.
#[derive(Clone)]
pub struct PassThroughVerifier {
    finalized: bool,
    fork_choice: ForkChoiceStrategy,
}

impl PassThroughVerifier {
    fn new(finalized: bool) -> Self {
        Self {
            finalized,
            fork_choice: ForkChoiceStrategy::LongestChain,
        }
    }

    fn new_with_fork_choice(finalized: bool, fork_choice: ForkChoiceStrategy) -> Self {
        Self {
            finalized,
            fork_choice,
        }
    }
}

#[async_trait::async_trait]
impl<B> Verifier<B> for PassThroughVerifier
where
    B: Block,
{
    async fn verify(
        &mut self,
        origin: BlockOrigin,
        header: B::Header,
        justifications: Option<Justifications>,
        body: Option<Vec<B::Extrinsic>>,
    ) -> Result<(BlockImportParams<B, ()>, Option<Vec<(CacheKeyId, Vec<u8>)>>), String> {
        todo!()
    }
}
