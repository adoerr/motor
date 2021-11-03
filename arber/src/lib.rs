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

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};

pub use pallet::*;

mod mmr;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub trait LeafProvider {
    type Leaf: Clone + Decode + Encode;

    fn leaf() -> Self::Leaf;
}

impl LeafProvider for () {
    type Leaf = ();

    fn leaf() -> Self::Leaf {}
}

type Leaf<T> = <<T as Config>::Leaf as LeafProvider>::Leaf;

type MMR<T> = mmr::MMR<T, Leaf<T>, mmr::Storage<T, Leaf<T>>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Prefix wich will be prepended to each offchain DB key.
        const KEY_PREFIX: &'static [u8];

        /// MMR leaf type
        type Leaf: LeafProvider;
    }

    #[pallet::storage]
    #[pallet::getter(fn root)]
    pub type Root<T> = StorageValue<_, (arber::Hash, u64), ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: T::BlockNumber) -> Weight {
            let (hash, size) = Self::root();

            sp_tracing::debug!(target: "arber", "⛰️ block_number: {} - root: {:?} - size: {}", block_number, hash, size);

            let data = T::Leaf::leaf();

            let mut mmr: MMR<T> = mmr::MMR::new(size);

            let _ = mmr.append(&data).expect("MMR append never fails");

            100_u64
        }
    }
}

impl<T: Config> Pallet<T> {
    // Map a MMR Store index to an offchain DB key
    fn storage_key(idx: u64) -> Vec<u8> {
        (T::KEY_PREFIX, idx).encode()
    }
}
