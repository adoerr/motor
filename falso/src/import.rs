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

use std::collections::HashMap;

use sc_service::Arc;
use sp_blockchain::well_known_cache_keys;
use sp_consensus::{
    import_queue::{CacheKeyId, Verifier},
    BlockImportParams, BlockOrigin, ForkChoiceStrategy,
};
use sp_runtime::{
    generic::OpaqueDigestItemId,
    traits::{Block, Header},
    Justifications,
};

use futures::lock::Mutex as AsyncMutex;
use parking_lot::Mutex;

/// A Verifier that accepts all justifications and passes them on for import.
///
/// Block finality and fork choice strategy are configurable.
#[derive(Clone)]
pub struct PassThroughVerifier {
    finalized: bool,
    fork_choice: ForkChoiceStrategy,
}

impl PassThroughVerifier {
    pub fn new(finalized: bool) -> Self {
        Self {
            finalized,
            fork_choice: ForkChoiceStrategy::LongestChain,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_fork_choice(finalized: bool, fork_choice: ForkChoiceStrategy) -> Self {
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
        let maybe_keys = header
            .digest()
            .log(|l| l.try_as_raw(OpaqueDigestItemId::Consensus(b"smpl")))
            .map(|l| vec![(well_known_cache_keys::AUTHORITIES, l.to_vec())]);

        let mut import = BlockImportParams::new(origin, header);
        import.body = body;
        import.finalized = self.finalized;
        import.justifications = justifications;
        import.fork_choice = Some(self.fork_choice);

        Ok((import, maybe_keys))
    }
}

/// Verifier implementation for tracking failed verifications
pub(crate) struct TrackingVerifier<B>
where
    B: Block,
{
    inner: Arc<AsyncMutex<Box<dyn Verifier<B>>>>,
    failed: Arc<Mutex<HashMap<B::Hash, String>>>,
}

impl<B> TrackingVerifier<B>
where
    B: Block,
{
    pub(crate) fn new(verifier: impl Verifier<B> + 'static) -> Self {
        TrackingVerifier {
            inner: Arc::new(AsyncMutex::new(Box::new(verifier))),
            failed: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl<B> Verifier<B> for TrackingVerifier<B>
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
        let hash = header.hash();

        self.inner
            .lock()
            .await
            .verify(origin, header, justifications, body)
            .await
            .map_err(|e| {
                self.failed.lock().insert(hash, e.clone());
                e
            })
    }
}

impl<B> Clone for TrackingVerifier<B>
where
    B: Block,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            failed: self.failed.clone(),
        }
    }
}
