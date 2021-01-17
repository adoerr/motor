use sc_cli::{ChainSpec, RuntimeVersion, SubstrateCli};

use crate::command::Command;
use crate::{chain_spec, service};

impl SubstrateCli for Command {
    fn impl_name() -> String {
        "Albert Node".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "Albert is not very supportive".into()
    }

    fn copyright_start_year() -> i32 {
        2020
    }

    fn load_spec(&self, _: &str) -> Result<Box<dyn ChainSpec>, String> {
        Ok(Box::new(chain_spec::dev_config()?))
    }

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &albert_runtime::VERSION
    }
}

pub fn run() -> sc_cli::Result<()> {
    let cli = Command::from_args();

    let mut runner = cli.create_runner(&cli.run)?;
    runner.config_mut().prometheus_config = None;
    runner.config_mut().telemetry_endpoints = None;
    
    runner.run_node_until_exit(|config| async move {
        service::new_full(config).map_err(sc_cli::Error::Service)
    })
}
