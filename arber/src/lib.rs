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

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub trait LeafProvider {
    type Leaf: Decode + Encode;

    fn leaf() -> Self::Leaf;
}

impl LeafProvider for () {
    type Leaf = ();

    fn leaf() -> Self::Leaf {}
}

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Hash: sp_std::hash::Hash
            + sp_std::fmt::Display
            + Default
            + Decode
            + Encode
            + scale_info::TypeInfo
            + codec::EncodeLike;
    }

    #[pallet::storage]
    #[pallet::getter(fn root)]
    pub type Root<T> = StorageValue<_, (<T as Config>::Hash, u64), ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: T::BlockNumber) -> Weight {
            let (hash, size) = Self::root();

            sp_tracing::debug!(target: "arber", "⛰ block_number: {} - root: {} - size: {}", block_number, hash, size);

            100_u64
        }
    }
}
