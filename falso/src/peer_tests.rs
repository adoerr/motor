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

use super::PeerConfig;

use crate::network::{Network, NetworkProvider};

#[tokio::test]
async fn add_single_block() {
    sp_tracing::try_init_simple();

    let mut net = Network::new();

    net.add_peer(PeerConfig::default());
    net.peer(0).add_block();

    let best = net.peer(0).client().info().best_number;

    assert_eq!(1, best);
}

#[tokio::test]
async fn add_multiple_blocks() {
    sp_tracing::try_init_simple();

    let mut net = Network::new();

    net.add_peer(PeerConfig::default());

    let hash = net.peer(0).add_blocks(5);

    net.block_until_synced();

    let best = net.peer(0).client().info().best_hash;

    assert_eq!(hash, best);
}
