use std::sync::Arc;

use sc_consensus::LongestChain;

use substrate_test_runtime::Block;
use substrate_test_runtime_client::{Backend, TestClient, TestClientBuilder, TestClientBuilderExt};

#[derive(Clone)]
pub struct Client {
    pub(crate) inner: Arc<TestClient>,
    pub(crate) backend: Arc<Backend>,
    pub(crate) chain: LongestChain<substrate_test_runtime_client::Backend, Block>,
}

impl Client {
    pub fn new() -> Client {
        let backend = Arc::new(Backend::new_test(std::u32::MAX, std::u64::MAX));
        let builder = TestClientBuilder::with_backend(backend);
        let backend = builder.backend();

        let (client, chain) = builder.build_with_longest_chain();

        Client {
            inner: Arc::new(client),
            backend,
            chain,
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Client;

    use sp_consensus::block_import::BlockOrigin;
    use sp_runtime::{ConsensusEngineId, Justification, Justifications};

    use sc_block_builder::BlockBuilderProvider;
    use sc_client_api::HeaderBackend;

    use substrate_test_runtime_client::prelude::*;

    use futures::executor::{self};

    #[test]
    fn import() {
        sp_tracing::try_init_simple();

        let mut client = Client::new();

        let block = client
            .inner
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        executor::block_on(client.inner.import(BlockOrigin::File, block)).unwrap();

        let info = client.inner.info();

        assert_eq!(1, info.best_number);
        assert_eq!(0, info.finalized_number);
    }

    #[test]
    fn import_blocks() {
        sp_tracing::try_init_simple();

        let mut client = Client::new();

        for _ in 0..10 {
            let block = client
                .inner
                .new_block(Default::default())
                .unwrap()
                .build()
                .unwrap()
                .block;

            executor::block_on(client.inner.import(BlockOrigin::File, block)).unwrap();
        }

        let info = client.inner.info();

        assert_eq!(10, info.best_number);
        assert_eq!(0, info.finalized_number);
    }

    #[test]
    fn import_finalized() {
        sp_tracing::try_init_simple();

        let mut client = Client::new();

        let block = client
            .inner
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        executor::block_on(client.inner.import_as_final(BlockOrigin::File, block)).unwrap();

        let info = client.inner.info();

        assert_eq!(1, info.best_number);
        assert_eq!(1, info.finalized_number);
    }

    #[test]
    fn import_justification() {
        sp_tracing::try_init_simple();

        const ENGINE_ID: ConsensusEngineId = *b"SMPL";

        let mut client = Client::new();

        let block = client
            .inner
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        let j: Justification = (ENGINE_ID, vec![1, 2, 3]);

        let j = Justifications::from(j);

        executor::block_on(client.inner.import_justified(BlockOrigin::File, block, j)).unwrap();

        let info = client.inner.info();

        assert_eq!(1, info.best_number);
        assert_eq!(1, info.finalized_number);
    }
}
