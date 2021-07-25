use substrate_test_runtime_client::TestClient;

pub type Client = TestClient;

pub fn new() -> Client {
    substrate_test_runtime_client::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    use sp_consensus::block_import::BlockOrigin;
    use sp_runtime::{ConsensusEngineId, Justification, Justifications};

    use sc_block_builder::BlockBuilderProvider;
    use sc_client_api::HeaderBackend;

    use substrate_test_runtime_client::prelude::*;

    use futures::executor::{self};

    #[test]
    fn import() {
        sp_tracing::try_init_simple();

        let mut client = new();

        let block = client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        executor::block_on(client.import(BlockOrigin::File, block)).unwrap();

        let info = client.info();

        assert_eq!(1, info.best_number);
        assert_eq!(0, info.finalized_number);
    }

    #[test]
    fn import_blocks() {
        sp_tracing::try_init_simple();

        let mut client = new();

        for _ in 0..10 {
            let block = client
                .new_block(Default::default())
                .unwrap()
                .build()
                .unwrap()
                .block;

            executor::block_on(client.import(BlockOrigin::File, block)).unwrap();
        }

        let info = client.info();

        assert_eq!(10, info.best_number);
        assert_eq!(0, info.finalized_number);
    }

    #[test]
    fn import_finalized() {
        sp_tracing::try_init_simple();

        let mut client = new();

        let block = client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        executor::block_on(client.import_as_final(BlockOrigin::File, block)).unwrap();

        let info = client.info();

        assert_eq!(1, info.best_number);
        assert_eq!(1, info.finalized_number);
    }

    #[test]
    fn import_justification() {
        sp_tracing::try_init_simple();

        const ENGINE_ID: ConsensusEngineId = *b"SMPL";

        let mut client = new();

        let block = client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        let j: Justification = (ENGINE_ID, vec![1, 2, 3]);

        let j = Justifications::from(j);

        executor::block_on(client.import_justified(BlockOrigin::File, block, j)).unwrap();

        let info = client.info();

        assert_eq!(1, info.best_number);
        assert_eq!(1, info.finalized_number);
    }
}
