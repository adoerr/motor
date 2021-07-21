use sp_core::H256;
use sp_runtime::traits::BlakeTwo256;
use sp_state_machine::{MemoryDB, TrieDBMut, TrieMut};

pub fn prepare_changes(changes: Vec<(Vec<u8>, Vec<u8>)>) -> (H256, MemoryDB<BlakeTwo256>) {
    let mut root = H256::default();
    let mut trie_update = MemoryDB::<BlakeTwo256>::default();

    {
        let mut trie = TrieDBMut::<BlakeTwo256>::new(&mut trie_update, &mut root);

        for (key, val) in changes {
            trie.insert(&key, &val)
                .expect("Trie K/V pair insert failed");
        }
    }

    (root, trie_update)
}
