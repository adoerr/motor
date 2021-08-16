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

use super::{Worker, WorkerParams};

use falso::{Network, NetworkProvider, PeerConfig};

use futures::{executor, future, task::Poll};
use tokio::{task, time};

#[tokio::test]
async fn idle_worker() {
    sp_tracing::try_init_simple();

    let mut net = Network::new();

    net.add_peer(PeerConfig::default());

    let peer = net.peer(0);

    let params = WorkerParams {
        client: peer.client().as_inner(),
        backend: peer.client().as_backend(),
    };

    let worker = task::spawn(async {
        let mut worker = Worker::new(params);
        let _ = worker.run().await;
    });

    peer.add_blocks(5);

    executor::block_on(future::poll_fn(move |cx| {
        net.poll(cx);
        Poll::Ready(())
    }));

    // give the worker a chance to acutally run
    time::sleep(time::Duration::from_millis(50)).await;

    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
}
