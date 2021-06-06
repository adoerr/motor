use sc_service::ChainType;
use sp_keyring::AccountKeyring;

use simplex_runtime::{BalancesConfig, GenesisConfig, SudoConfig, SystemConfig, WASM_BINARY};

pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Chain specification configuration for development
pub fn dev_config() -> Result<ChainSpec, String> {
    let wasm =
        WASM_BINARY.ok_or_else(|| "Wasm binary development version not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Development",
        // Id
        "dev",
        // Chain type
        ChainType::Development,
        // Genesis source
        move || genesis(wasm),
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol Id
        Some("mtr"),
        // Properties
        None,
        // Extensions
        None,
    ))
}

/// Genesis configuration
fn genesis(wasm: &[u8]) -> GenesisConfig {
    // sudo account
    let sudo_key = AccountKeyring::Alice.to_account_id();
    // pre-funded accounts
    let accounts = vec![
        AccountKeyring::Alice.to_account_id(),
        AccountKeyring::Bob.to_account_id(),
        AccountKeyring::Charlie.to_account_id(),
    ];

    GenesisConfig {
        frame_system: SystemConfig {
            // store wasm runtime
            code: wasm.to_vec(),
            changes_trie_config: Default::default(),
        },
        pallet_balances: BalancesConfig {
            // initial account balances
            balances: accounts.iter().cloned().map(|acc| (acc, 1 << 60)).collect(),
        },
        pallet_sudo: SudoConfig { key: sudo_key },
    }
}
