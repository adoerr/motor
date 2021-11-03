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

pub struct Storage<T, L>(PhantomData<(T, L)>);

impl<T, L> Default for Storage<T, L> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T, L> Store<L> for Storage<T, L>
where
    T: Config,
    L: Clone + Decode + Encode,
{
    fn hash_at(&self, idx: u64) -> arber::Result<arber::Hash> {
        let key = Pallet::<T>::storage_key(idx);

        let hash =
            local_storage_get(StorageKind::LOCAL, &key).ok_or(Error::MissingHashAtIndex(idx))?;

        let hash: arber::Hash =
            Decode::decode(&mut hash.as_ref()).map_err(|_| Error::MissingHashAtIndex(idx))?;

        Ok(hash)
    }

    fn append(&mut self, _elem: &L, hashes: &[arber::Hash]) -> arber::Result<()> {
        let (_, mut size) = Pallet::<T>::root();

        for h in hashes {
            let key = Pallet::<T>::storage_key(size);
            sp_io::offchain_index::set(&key, h.as_ref());
            size += 1;
        }

        Ok(())
    }
}

#[allow(dead_code, clippy::upper_case_acronyms)]
pub struct MMR<T, L, S>
where
    T: Config,
    L: Clone + Decode + Encode,
    S: Store<L>,
{
    mmr: MerkleMountainRange<L, S>,
    size: u64,
    _config: PhantomData<T>,
}

impl<T, L, S> MMR<T, L, S>
where
    T: Config,
    L: Clone + Decode + Encode,
    S: Store<L> + Default,
{
    pub fn new(size: u64) -> Self {
        Self {
            mmr: MerkleMountainRange::new(size, Default::default()),
            size,
            _config: PhantomData,
        }
    }

    pub fn append(&mut self, elem: &L) -> arber::Result<u64> {
        let _ = self.mmr.append(elem)?;

        let root = self.mmr.root()?;
        let size = self.mmr.size();

        Root::<T>::put((root, size));

        Ok(size)
    }
}
