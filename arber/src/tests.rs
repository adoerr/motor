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

use crate::mock::MockRuntime;

fn new_test_ext() -> TestExternalities {
    frame_system::GenesisConfig::default()
        .build_storage::<MockRuntime>()
        .unwrap()
        .into()
}

fn register_offchain_ext(ext: &mut TestExternalities) {
    let (off_ext, _) = TestOffchainExt::with_offchain_db(ext.offchain_db());

    ext.register_extension(OffchainDbExt::new(off_ext.clone()));
    ext.register_extension(OffchainWorkerExt::new(off_ext));
}
