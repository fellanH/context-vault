//! context_for, list_situations, vault_map: the situation index and the generated overview.
//! Results are JSON text, like the hosted tools of the same names.

use std::sync::Arc;
use serde_json::Value;
use crate::engine::VaultEngine;

fn pretty(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}

/// Implements context_for: one page of a situation's entries, in order, with bodies.
pub async fn handle_context_for(engine: Arc<dyn VaultEngine>, args: &Value) -> Result<String, String> {
    let situation = args.get("situation").and_then(|v| v.as_str()).ok_or("Required: situation")?;
    let cursor = args.get("cursor").and_then(|v| v.as_str());
    let page = engine.read_situation(situation, cursor).await.map_err(|e| e.to_string())?;
    pretty(&page)
}

/// Implements list_situations: situation names with live entry counts.
pub async fn handle_list_situations(engine: Arc<dyn VaultEngine>) -> Result<String, String> {
    pretty(&engine.list_situations().await.map_err(|e| e.to_string())?)
}

/// Implements list_vaults: the vaults this engine serves (v5: the registry).
pub async fn handle_list_vaults(engine: Arc<dyn VaultEngine>) -> Result<String, String> {
    pretty(&engine.list_vaults().await.map_err(|e| e.to_string())?)
}

/// Implements vault_map: the cached overview, regenerated when stale or on `refresh`.
pub async fn handle_vault_map(engine: Arc<dyn VaultEngine>, args: &Value) -> Result<String, String> {
    let force = args.get("refresh").and_then(|v| v.as_bool()).unwrap_or(false);
    pretty(&engine.vault_map(force).await.map_err(|e| e.to_string())?)
}
