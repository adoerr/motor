use std::sync::Arc;

use sc_executor::native_executor_instance;
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, TaskManager};
use sp_api::TransactionFor;
use sp_consensus::import_queue::BasicQueue;

use motor_runtime::{self, opaque::Block, RuntimeApi};

use simplex::SimplexConfig;

pub use sc_executor::NativeExecutor;

native_executor_instance!(
    pub Executor,
    motor_runtime::api::dispatch,
    motor_runtime::native_version,
);

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectedChain = sc_consensus::LongestChain<FullBackend, Block>;

type ServiceComponents = sc_service::PartialComponents<
    FullClient,
    FullBackend,
    FullSelectedChain,
    BasicQueue<Block, TransactionFor<FullClient, Block>>,
    sc_transaction_pool::FullPool<Block, FullClient>,
    (),
>;

pub fn new_partial(config: &Configuration) -> Result<ServiceComponents, ServiceError> {
    // create full node initial parts
    let (client, backend, keystore_container, task_manager, ..) =
        sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;

    let client = Arc::new(client);

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        None,
        task_manager.spawn_handle(),
        client.clone(),
    );

    let inherent_data_providers = sp_inherents::InherentDataProviders::new();

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let config = SimplexConfig {
        block_authority: sp_keyring::AccountKeyring::Alice.public().into(),
        finality_authority: sp_keyring::AccountKeyring::Bob.public().into(),
    };

    let import_queue = simplex::import_queue(
        config,
        client.clone(),
        client.clone(),
        &task_manager.spawn_handle(),
    );

    Ok(PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        inherent_data_providers,
        other: (),
    })
}

/// Bootstrap services for a new full client
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
    let PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        ..
    } = new_partial(&config)?;

    let (network, network_status_sinks, system_rpc_tx, network_starter) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            on_demand: None,
            block_announce_validator_builder: None,
        })?;

    if config.offchain_worker.enabled {
        sc_service::build_offchain_workers(
            &config,
            backend.clone(),
            task_manager.spawn_handle(),
            client.clone(),
            network.clone(),
        );
    }

    let role = config.role.clone();

    let rpc_extensions_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();

        Box::new(move |deny_unsafe, _| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                deny_unsafe,
            };

            crate::rpc::create_full(deps)
        })
    };

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        network: network.clone(),
        client: client.clone(),
        keystore: keystore_container.sync_keystore(),
        task_manager: &mut task_manager,
        transaction_pool: transaction_pool.clone(),
        telemetry_span: None,
        rpc_extensions_builder,
        on_demand: None,
        remote_blockchain: None,
        backend,
        network_status_sinks,
        system_rpc_tx,
        config,
    })?;

    if role.is_authority() {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            None,
        );

        simplex::start_simplex(
            client.clone(),
            select_chain,
            client.clone(),
            proposer,
            network.clone(),
            sp_keyring::AccountKeyring::Alice.pair().into(),
        );
    }

    network_starter.start_network();

    Ok(task_manager)
}
