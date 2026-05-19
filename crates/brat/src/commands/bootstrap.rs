//! Bootstrap command handlers.

use crate::cli::{Cli, BootstrapCommand, BootstrapRunArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Run a bootstrap subcommand.
pub fn run(cli: &Cli, cmd: &BootstrapCommand) -> Result<(), BratError> {
    match cmd {
        BootstrapCommand::Run(args) => run_bootstrap(cli, args),
    }
}

fn run_bootstrap(cli: &Cli, args: &BootstrapRunArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;

    print_human(cli, "Starting bootstrap...");
    print_human(cli, &format!("Max iterations: {}", args.max_iterations));

    // TODO: Integrate with libbrat_engine::bootstrap
    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            consistent: bool,
            score: u8,
            iterations: u32,
            inconsistencies: Vec<String>,
        }
        output_success(cli, Output {
            consistent: false,
            score: 0,
            iterations: 0,
            inconsistencies: vec![],
        });
    } else {
        print_human(cli, "Bootstrap integration pending.");
    }

    Ok(())
}
