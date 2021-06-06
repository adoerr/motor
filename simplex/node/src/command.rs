use structopt::StructOpt;
use sc_cli::RunCmd;

#[derive(Debug, StructOpt)]
pub struct Command {
    #[structopt(flatten)]
    pub run: RunCmd,
}