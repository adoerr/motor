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

use sp_core::offchain::StorageKind;
use sp_io::offchain::local_storage_get;
use sp_std::marker::PhantomData;

use arber::{Error, MerkleMountainRange, Store};
use codec::{Decode, Encode};

use crate::{Config, Pallet, Root};

/// Marker type for a runtime-specific storage implementation
pub(crate) struct Runtime;

/// Marker type for an offchain-specific storage implementation
pub(crate) struct Offchain;

pub struct Storage<S, T, L>(PhantomData<(S, T, L)>);

impl<T, L> Store<L> for Storage<Runtime, T, L>
where
    T: Config,
    L: Clone + Decode + Encode,
{
    fn hash_at(&self, index: u64) -> arber::Result<arber::Hash> {
        todo!()
    }

    fn append(&mut self, elem: &L, hashes: &[arber::Hash]) -> arber::Result<()> {
        todo!()
    }
}
