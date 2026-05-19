//! MCP server command handler.

use crate::cli::{Cli, McpArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Run the MCP server command.
pub async fn run(cli: &Cli, _args: &McpArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;

    print_human(cli, "Starting MCP server...");

    // TODO: Integrate with brat_mcp crate
    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            status: String,
        }
        output_success(cli, Output {
            status: "pending".to_string(),
        });
    } else {
        print_human(cli, "MCP server integration pending.");
    }

    Ok(())
}
