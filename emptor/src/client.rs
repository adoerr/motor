#[allow(unused_imports)]
use substrate_test_runtime_client::{prelude::*, TestClient};

pub type Client = TestClient;

pub fn new() -> Client {
    substrate_test_runtime_client::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    use sp_consensus::block_import::BlockOrigin;

    use sc_block_builder::BlockBuilderProvider;
    use sc_client_api::HeaderBackend;

    use futures::executor::{self};

    #[test]
    fn import_block() {
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
}
