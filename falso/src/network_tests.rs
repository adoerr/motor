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

use super::{Network, NetworkProvider, PeerConfig};

#[tokio::test]
async fn new_network() {
    sp_tracing::try_init_simple();

    let mut net = Network::new();

    assert_eq!(net.peers.len(), 0);

    net.add_peer(PeerConfig::default());
    net.add_peer(PeerConfig::default());

    assert_eq!(net.peers.len(), 2);

    let id1 = net.peer(0).id();
    let id2 = net.peer(1).id();

    assert_ne!(id1, id2);
    assert_eq!(0, net.peer(0).connected_peers());
    assert_eq!(0, net.peer(1).connected_peers());
}

#[tokio::test]
async fn connect_all_peers() {
    sp_tracing::try_init_simple();

    let mut net = Network::new();

    for _ in 0..5 {
        net.add_peer(PeerConfig::default());
    }

    assert!(net.peers().iter().all(|p| p.connected_peers() == 0));

    net.block_until_connected();

    let others = net.peers().len() - 1;

    assert!(net.peers().iter().all(|p| p.connected_peers() == others));
}
