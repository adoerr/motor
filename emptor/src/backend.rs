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

#![allow(dead_code)]

use sc_client_api::{Backend, BlockImportOperation, NewBlockState};
use sp_core::H256;
use sp_runtime::{
    generic::{BlockId, DigestItem},
    testing::{Digest, ExtrinsicWrapper, Header},
    traits::{BlakeTwo256, Hash},
};
use sp_state_machine::{ChangesTrieCacheAction, IndexOperation, MemoryDB, TrieDBMut, TrieMut};

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;

type Block = sp_runtime::testing::Block<ExtrinsicWrapper<u64>>;

pub fn changes_trie(changes: Vec<(Vec<u8>, Vec<u8>)>) -> (H256, MemoryDB<BlakeTwo256>) {
    let mut root = H256::default();
    let mut trie_update = MemoryDB::<BlakeTwo256>::default();

    {
        let mut trie = TrieDBMut::<BlakeTwo256>::new(&mut trie_update, &mut root);

        for (key, val) in changes {
            trie.insert(&key, &val)
                .expect("trie k/v pair insert failed");
        }
    }

    (root, trie_update)
}

pub fn insert_header(
    backend: &sc_client_db::Backend<Block>,
    number: u64,
    parent_hash: H256,
    changes: Option<Vec<(Vec<u8>, Vec<u8>)>>,
    extrinsics_root: H256,
) -> H256 {
    insert_block(
        backend,
        number,
        parent_hash,
        changes,
        extrinsics_root,
        Vec::new(),
        None,
    )
}

pub fn insert_block(
    backend: &sc_client_db::Backend<Block>,
    number: u64,
    parent_hash: H256,
    changes: Option<Vec<(Vec<u8>, Vec<u8>)>>,
    extrinsics_root: H256,
    body: Vec<ExtrinsicWrapper<u64>>,
    transaction_idx: Option<Vec<IndexOperation>>,
) -> H256 {
    let mut digest = Digest::default();
    let mut changes_trie_update = Default::default();

    if let Some(changes) = changes {
        let (root, update) = changes_trie(changes);

        digest.push(DigestItem::ChangesTrieRoot(root));
        changes_trie_update = update;
    }

    let header = Header {
        parent_hash,
        number,
        state_root: BlakeTwo256::trie_root(Vec::new()),
        extrinsics_root,
        digest,
    };

    let hash = header.hash();

    let block_id = if number == 0 {
        BlockId::Hash(Default::default())
    } else {
        BlockId::Number(number - 1)
    };

    let mut op = backend
        .begin_operation()
        .expect("begin block insert operation failed");

    backend
        .begin_state_operation(&mut op, block_id)
        .expect("note state transition failed");

    op.set_block_data(header, Some(body), None, None, NewBlockState::Best)
        .expect("append block data failed");

    if let Some(idx) = transaction_idx {
        op.update_transaction_index(idx)
            .expect("add transaction idx failed");
    }

    op.update_changes_trie((changes_trie_update, ChangesTrieCacheAction::Clear))
        .expect("update trie data failed");

    backend.commit_operation(op).expect("block insert failed");

    hash
}
