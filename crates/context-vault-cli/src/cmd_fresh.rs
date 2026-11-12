//! `stale [--kind]` and `verify <id>`: freshness windows from vault.toml `fresh_days` (card 28).

use anyhow::{anyhow, Result};
use context_vault_core::engine::SaveInput;
use context_vault_core::paths::resolve_vault_dir;
use cv_vault::VaultTaxonomy;
use serde_json::{json, Value};

use crate::engine_cmd;

pub fn run_stale(kind: Option<String>) -> Result<()> {
    let vault_dir = resolve_vault_dir()?;
    let taxonomy = VaultTaxonomy::load(&vault_dir);
    let found = cv_vault::stale::stale(&vault_dir, &taxonomy, kind.as_deref(), chrono::Utc::now())?;
    for e in &found {
        println!(
            "{}  {}  {}d old (fresh {}d, last {})  {}",
            e.id.as_deref().unwrap_or("-"),
            e.kind,
            e.days_old,
            e.fresh_days,
            e.last_checked,
            e.title.as_deref().unwrap_or("")
        );
    }
    eprintln!("{} stale", found.len());
    Ok(())
}

/// Stamp `meta.verified_at = now`: the entry was checked and still holds.
pub async fn run_verify(id: &str) -> Result<()> {
    let engine = engine_cmd::build_engine().await?;
    let e = engine.get(id).await?.ok_or_else(|| anyhow!("entry not found: {id}"))?;
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut meta = match e.meta {
        Some(Value::Object(m)) => m,
        _ => serde_json::Map::new(),
    };
    meta.insert("verified_at".into(), json!(now));
    engine
        .save(SaveInput {
            id: Some(e.id.clone()),
            kind: e.kind,
            title: e.title,
            body: e.body,
            tags: Some(e.tags),
            meta: Some(Value::Object(meta)),
            source: e.source,
            identity_key: e.identity_key,
            tier: Some(e.tier),
            expires_at: e.expires_at,
            supersedes: None,
            related_to: e.related_to,
        })
        .await?;
    println!("verified: {} at {now}", e.id);
    Ok(())
}
