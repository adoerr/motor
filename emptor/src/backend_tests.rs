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

use super::{insert_header, Block};

use sc_client_api::Backend;
use sp_blockchain::{Backend as ChainBackend, HeaderBackend};
use sp_runtime::{generic::BlockId, ConsensusEngineId, Justification, Justifications};

#[test]
fn insert_headers() {
    let backend = sc_client_db::Backend::<Block>::new_test(1, 0);

    for i in 0..10 {
        assert!(backend.blockchain().hash(i).unwrap().is_none());

        insert_header(
            &backend,
            i,
            if i == 0 {
                Default::default()
            } else {
                backend.blockchain().hash(i - 1).unwrap().unwrap()
            },
            None,
            Default::default(),
        );

        assert!(backend.blockchain().hash(i).unwrap().is_some());
    }

    assert_eq!(9, backend.blockchain().info().best_number);
}

#[test]
fn prunning_leaves() {
    let backend = sc_client_db::Backend::<Block>::new_test(10, 10);

    // height 0 - genesis
    let b_0 = insert_header(&backend, 0, Default::default(), None, Default::default());

    // height 1
    let b_0_1 = insert_header(&backend, 1, b_0, None, Default::default());
    let b_0_2 = insert_header(&backend, 1, b_0, None, [1; 32].into());
    let b_0_3 = insert_header(&backend, 1, b_0, None, [2; 32].into());

    assert_eq!(
        vec![b_0_1, b_0_2, b_0_3],
        backend.blockchain().leaves().unwrap()
    );

    // height 2
    let b_0_1_1 = insert_header(&backend, 2, b_0_1, None, Default::default());
    let b_0_2_1 = insert_header(&backend, 2, b_0_2, None, Default::default());
    let b_0_2_2 = insert_header(&backend, 2, b_0_2, None, [1; 32].into());

    assert_eq!(
        vec![b_0_1_1, b_0_2_1, b_0_2_2, b_0_3],
        backend.blockchain().leaves().unwrap()
    );

    backend.finalize_block(BlockId::Hash(b_0_1), None).unwrap();

    backend
        .finalize_block(BlockId::Hash(b_0_1_1), None)
        .unwrap();

    // leaves at the same height stay, leaves at lower height get pruned (`b_0_3`)
    assert_eq!(
        vec![b_0_1_1, b_0_2_1, b_0_2_2],
        backend.blockchain().leaves().unwrap()
    );
}

#[test]
fn finalize_with_justification() {
    const ENGINE_ID: ConsensusEngineId = *b"SMPL";

    let backend = sc_client_db::Backend::<Block>::new_test(10, 10);

    let b_0 = insert_header(&backend, 0, Default::default(), None, Default::default());
    let b_1 = insert_header(&backend, 1, b_0, None, Default::default());

    let j: Justification = (ENGINE_ID, vec![1, 2, 3]);

    backend
        .finalize_block(BlockId::Hash(b_1), Some(j.clone()))
        .unwrap();

    assert_eq!(
        Justifications::from(j),
        backend
            .blockchain()
            .justifications(BlockId::Hash(b_1))
            .unwrap()
            .unwrap()
    );
}

#[test]
fn append_justifications() {
    const CONS_ENGINE_0: ConsensusEngineId = *b"SMPL";
    const CONS_ENGINE_1: ConsensusEngineId = *b"BEEF";

    let backend = sc_client_db::Backend::<Block>::new_test(10, 10);

    let b_0 = insert_header(&backend, 0, Default::default(), None, Default::default());
    let b_1 = insert_header(&backend, 1, b_0, None, Default::default());

    let j_0: Justification = (CONS_ENGINE_0, vec![1, 2, 3]);

    backend
        .finalize_block(BlockId::Hash(b_1), Some(j_0.clone()))
        .unwrap();

    let j_1: Justification = (CONS_ENGINE_1, vec![4, 5, 6]);

    backend
        .append_justification(BlockId::Hash(b_1), j_1.clone())
        .unwrap();

    let j_2: Justification = (CONS_ENGINE_1, vec![7, 8, 9]);

    assert!(matches!(
        backend.append_justification(BlockId::Hash(b_1), j_2),
        Err(sp_blockchain::Error::BadJustification(_))
    ));

    let justifications = {
        let mut j = Justifications::from(j_0);
        j.append(j_1);
        j
    };

    assert_eq!(
        Some(justifications),
        backend
            .blockchain()
            .justifications(BlockId::Hash(b_1))
            .unwrap(),
    );
}
