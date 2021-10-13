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

use arber::{Error, Store};
use codec::Decode;

use crate::{Config, Pallet};

#[derive(Default)]
pub struct Storage<T>(PhantomData<T>);

impl<T> Store<Vec<u8>> for Storage<T>
where
    T: Config,
{
    fn append(&mut self, _elem: &Vec<u8>, _hashes: &[arber::Hash]) -> arber::Result<()> {
        todo!()
    }

    fn hash_at(&self, idx: u64) -> arber::Result<arber::Hash> {
        let key = Pallet::<T>::storage_key(idx);

        let hash =
            local_storage_get(StorageKind::LOCAL, &key).ok_or(Error::MissingHashAtIndex(idx))?;

        let hash: arber::Hash =
            Decode::decode(&mut hash.as_ref()).map_err(|_| Error::MissingHashAtIndex(idx))?;

        Ok(hash)
    }
}
