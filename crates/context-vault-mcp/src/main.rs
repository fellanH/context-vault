//! `context-vault-mcp` — dedicated MCP stdio server binary.
//!
//! Thin entrypoint over `context_vault_core::mcp::run()`. Equivalent to
//! `context-vault mcp`, exposed as its own binary for the per-platform
//! distribution (Phase 7) and MCP-client registration.

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    context_vault_core::mcp::run().await
}
