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

use sp_core::offchain::{testing::TestOffchainExt, OffchainDbExt, OffchainWorkerExt};
use sp_io::TestExternalities;
use sp_runtime::testing::Header;

use frame_support::{dispatch::Weight, traits::OnInitialize};

use crate::mock::{Arber, MockRuntime, LEAF};

fn new_test_ext() -> TestExternalities {
    frame_system::GenesisConfig::default()
        .build_storage::<MockRuntime>()
        .unwrap()
        .into()
}

#[allow(dead_code)]
fn register_offchain_ext(ext: &mut TestExternalities) {
    let (off_ext, _) = TestOffchainExt::with_offchain_db(ext.offchain_db());

    ext.register_extension(OffchainDbExt::new(off_ext.clone()));
    ext.register_extension(OffchainWorkerExt::new(off_ext));
}

#[allow(dead_code)]
fn next_block() -> Weight {
    let number = frame_system::Pallet::<MockRuntime>::block_number() + 1;
    let parent_hash = LEAF.with(|l| l.borrow().header.hash());

    LEAF.with(|l| l.borrow_mut().header = Header::new_from_number(number));

    frame_system::Pallet::<MockRuntime>::initialize(
        &number,
        &parent_hash,
        &Default::default(),
        frame_system::InitKind::Full,
    );

    Arber::on_initialize(number)
}

#[test]
fn initialize_root_works() {
    sp_tracing::try_init_simple();

    let mut ext = new_test_ext();

    ext.execute_with(|| {
        let (hash, size) = crate::Root::<MockRuntime>::get();

        assert_eq!("000000000000".to_string(), format!("{}", hash));
        assert_eq!(0, size);
    });
}

#[test]
fn single_block_works() {
    sp_tracing::try_init_simple();

    let mut ext = new_test_ext();

    ext.execute_with(|| {
        let weight = next_block();

        assert_eq!(100, weight);
    })
}
