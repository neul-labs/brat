//! Brat MCP server.
//!
//! Exposes brat operations as MCP tools via stdio transport.
//!
//! Tools:
//! - brat_bootstrap
//! - brat_consistency_check
//! - brat_kb_search
//! - brat_kb_create
//! - brat_approve
//! - brat_reject

use rmcp::ServiceExt;
use tracing::info;

mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("brat-mcp starting...");

    let server = tools::BratMcpServer::new();
    let (stdin, stdout) = rmcp::transport::io::stdio();

    let running = server.serve((stdin, stdout)).await?;
    info!("brat-mcp server running, waiting for client...");

    running.waiting().await?;
    info!("brat-mcp server exited.");

    Ok(())
}
