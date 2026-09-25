//! MCP (Model Context Protocol) stdio server over the vault engine.
//!
//! `stdio::run()` is the in-process server: it constructs a `SqliteBackend`
//! (with a real embedder when the model loads, else a no-op) and serves
//! JSON-RPC over stdin/stdout. `tools` holds the dispatch, schemas, and
//! tool handlers.

pub mod stdio;
pub mod tools;

// Module declarations for handler organization (child modules of tools)
pub(super) mod tool_schemas;
pub(super) mod handlers_read;
pub(super) mod handlers_recall;
pub(super) mod handlers_write;
pub(super) mod handlers_ingest;

pub use stdio::run;

use std::sync::Arc;

use crate::engine::VaultEngine;

/// Public entry point for the `recall` capability so non-MCP callers (e.g. the CLI's
/// `recall` subcommand and the recall hooks) reuse the exact hint logic — keyword
/// extraction + hybrid search + hint formatting — instead of duplicating it. Wraps
/// `handlers_recall::handle_recall` with a minimal signal payload.
/// `scope` restricts hints to the named project bucket plus globally-tagged
/// entries (see `SearchOptions::scope_project`); `None` searches the whole vault.
pub async fn recall(
    engine: Arc<dyn VaultEngine>,
    signal: &str,
    scope: Option<&str>,
    max_hints: usize,
) -> Result<String, String> {
    let args = serde_json::json!({
        "signal": signal,
        "signal_type": "prompt",
        "bucket": scope,
        "max_hints": max_hints,
    });
    handlers_recall::handle_recall(engine, &args).await
}
