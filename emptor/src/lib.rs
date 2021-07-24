use sc_client_api::{Backend, BlockImportOperation, NewBlockState};
use sp_core::H256;
use sp_runtime::{
    generic::{BlockId, DigestItem},
    testing::{Digest, ExtrinsicWrapper, Header},
    traits::{BlakeTwo256, Hash},
};
use sp_state_machine::{ChangesTrieCacheAction, IndexOperation, MemoryDB, TrieDBMut, TrieMut};

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

#[cfg(test)]
mod tests {
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
}
