#![allow(dead_code)]

use sc_executor::native_executor_instance;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};

use simplex_runtime::{self, opaque::Block, RuntimeApi};

//use simplex::SimplexConfig;

pub use sc_executor::NativeExecutor;

native_executor_instance!(
    pub Executor,
    simplex_runtime::api::dispatch,
    simplex_runtime::native_version,
);

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectedChain = sc_consensus::LongestChain<FullBackend, Block>;

type ServiceComponents = sc_service::PartialComponents<
    FullClient,
    FullBackend,
    FullSelectedChain,
    sp_consensus::DefaultImportQueue<Block, FullClient>,
    sc_transaction_pool::FullPool<Block, FullClient>,
    (),
>;

pub fn new_partial(_config: &Configuration) -> Result<ServiceComponents, ServiceError> {
    todo!()
}

/// Bootstrap services for a new full client
pub fn new_full(_config: Configuration) -> Result<TaskManager, ServiceError> {
    todo!()
}
