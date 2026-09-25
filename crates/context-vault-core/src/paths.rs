//! Shared vault.db / vault-dir path resolution, used by the MCP server, the CLI, and
//! the gateway so all three agree on where an existing install's data lives.
//!
//! Precedence matches the legacy TS config (`packages/core/src/config.ts`, now
//! deleted): default < `config.json` < env var. Existing installs' `setup` wizard
//! writes `vaultDir`/`dbPath` into `<dataDir>/config.json` — skipping that file (as
//! the Rust ports originally did, each independently) silently resolves to the wrong
//! directory for anyone who customized it.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::Value;

/// `~/.context-mcp` by default, overridable by CONTEXT_VAULT_DATA_DIR / CONTEXT_MCP_DATA_DIR.
/// This is where `config.json` lives.
fn data_dir_path(home: &str) -> String {
    std::env::var("CONTEXT_VAULT_DATA_DIR")
        .or_else(|_| std::env::var("CONTEXT_MCP_DATA_DIR"))
        .unwrap_or_else(|_| PathBuf::from(home).join(".context-mcp").to_string_lossy().to_string())
}

fn read_config_json(home: &str) -> Option<Value> {
    let path = PathBuf::from(data_dir_path(home)).join("config.json");
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Resolve the vault.db path: CONTEXT_VAULT_DB_PATH / CONTEXT_MCP_DB_PATH env vars win
/// over `config.json`'s `dbPath`, which wins over the `~/.context-mcp/vault.db` default.
/// Ensures the parent directory exists regardless of which tier resolved the path.
pub fn resolve_db_path() -> Result<String> {
    let home = std::env::var("HOME").context("HOME not set")?;

    let path = if let Ok(p) =
        std::env::var("CONTEXT_VAULT_DB_PATH").or_else(|_| std::env::var("CONTEXT_MCP_DB_PATH"))
    {
        p
    } else if let Some(p) =
        read_config_json(&home).and_then(|c| c.get("dbPath")?.as_str().map(String::from))
    {
        p
    } else {
        PathBuf::from(&home).join(".context-mcp/vault.db").to_string_lossy().to_string()
    };

    if let Some(parent) = PathBuf::from(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(path)
}

/// Resolve the markdown vault directory (where `.md` files live): CONTEXT_VAULT_VAULT_DIR
/// / CONTEXT_MCP_VAULT_DIR env vars win over `config.json`'s `vaultDir`, which wins over
/// the `~/.vault` default. Ensures the directory exists regardless of which tier
/// resolved it.
pub fn resolve_vault_dir() -> Result<String> {
    let home = std::env::var("HOME").context("HOME not set")?;

    let path = if let Ok(p) = std::env::var("CONTEXT_VAULT_VAULT_DIR")
        .or_else(|_| std::env::var("CONTEXT_MCP_VAULT_DIR"))
    {
        p
    } else if let Some(p) =
        read_config_json(&home).and_then(|c| c.get("vaultDir")?.as_str().map(String::from))
    {
        p
    } else {
        PathBuf::from(&home).join(".vault").to_string_lossy().to_string()
    };

    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Resolve the embedding-model cache dir: `CONTEXT_VAULT_MODEL_CACHE_DIR` /
/// `CONTEXT_MCP_MODEL_CACHE_DIR` env vars win over the `<data_dir>/models/fastembed`
/// default.
///
/// A model is a re-downloadable cache, not user data, but this repo has exactly one
/// answer to "where does an install's stuff live" (`data_dir_path`, above) and no
/// current need to split cache from data into a second, platform-specific XDG-style
/// location — so it gets a subdirectory of the same root rather than a new convention.
/// This is passed into `fastembed::InitOptions.cache_dir` explicitly: fastembed's own
/// default for that field is the bare relative path `.fastembed_cache`, which resolves
/// against the process CWD instead of a stable per-user location. `HF_HOME`, if the
/// user has it set, still overrides this deeper in fastembed's call chain regardless
/// of what we pass — see `FastembedEmbedder::new`'s doc comment.
pub fn resolve_model_cache_dir() -> Result<String> {
    let home = std::env::var("HOME").context("HOME not set")?;

    let path = if let Ok(p) = std::env::var("CONTEXT_VAULT_MODEL_CACHE_DIR")
        .or_else(|_| std::env::var("CONTEXT_MCP_MODEL_CACHE_DIR"))
    {
        p
    } else {
        // "fastembed", not bare "models": an existing install's <data_dir>/models
        // may already hold an unrelated, differently-shaped cache from the deleted
        // TS engine's transformers.js embedder (Xenova/all-MiniLM-L6-v2, HF-hub-cache
        // format is different) — keep this cache's tree distinguishable from that.
        PathBuf::from(data_dir_path(&home)).join("models").join("fastembed").to_string_lossy().to_string()
    };

    std::fs::create_dir_all(&path)?;
    Ok(path)
}
