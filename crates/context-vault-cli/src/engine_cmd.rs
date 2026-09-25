//! Engine-facing CLI subcommands (reindex / status / doctor / vault). These build a
//! SqliteBackend from the resolved db + vault paths and drive the VaultEngine trait.
//! A NoOpEmbedder is used: these ops rebuild the FTS index / report state and never
//! need semantic embeddings, so we skip the (slow) model load.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use context_vault_core::backends::{NoOpEmbedder, SqliteBackend};
use context_vault_core::engine::{ReindexOptions, VaultEngine};
use context_vault_core::paths::{resolve_db_path, resolve_vault_dir};
use context_vault_core::schema::vaults;

pub async fn build_engine() -> Result<SqliteBackend> {
    let db = resolve_db_path()?;
    let vault = resolve_vault_dir()?;
    let engine = SqliteBackend::new(&db, &vault, Arc::new(NoOpEmbedder))?;
    engine.init().await?;
    Ok(engine)
}

pub async fn run_reindex(full: bool) -> Result<()> {
    let engine = build_engine().await?;
    let summary = engine.reindex(ReindexOptions { full }).await?;
    println!(
        "reindexed {} of {} files, removed {} rows for missing files, in {:.0}ms",
        summary.entries_reindexed, summary.files_seen, summary.rows_removed, summary.duration_ms
    );
    if let Some(why) = &summary.prune_refused {
        eprintln!("warning: {why}");
    }
    Ok(())
}

pub async fn run_status() -> Result<()> {
    let engine = build_engine().await?;
    let stats = engine.stats().await?;
    let health = engine.health().await?;
    println!("vault: {} ({})", if health.ok { "ok" } else { "unhealthy" }, health.message);
    println!("entries: {} total, {} indexed", stats.total_entries, stats.indexed_entries);
    println!("db:        {}", resolve_db_path()?);
    println!("vault dir: {}", resolve_vault_dir()?);
    println!("by kind:");
    for (kind, count) in &stats.by_kind {
        println!("  {}: {}", kind, count);
    }
    println!("by tier:");
    for (tier, count) in &stats.by_tier {
        println!("  {}: {}", tier, count);
    }
    Ok(())
}

/// Machine-readable counterpart to `status` — same stats, raw JSON, no health/paths.
pub async fn run_stats() -> Result<()> {
    let engine = build_engine().await?;
    let stats = engine.stats().await?;
    println!("{}", serde_json::to_string_pretty(&stats)?);
    Ok(())
}

pub async fn run_doctor() -> Result<()> {
    let db = resolve_db_path()?;
    let vault = resolve_vault_dir()?;
    let db_state = if Path::new(&db).exists() { "exists" } else { "missing" };
    let vault_state = if Path::new(&vault).is_dir() { "exists" } else { "missing" };
    println!("db path:   {db} ({db_state})");
    println!("vault dir: {vault} ({vault_state})");
    let engine = build_engine().await?;
    let health = engine.health().await?;
    println!("engine health: {} — {}", if health.ok { "OK" } else { "FAIL" }, health.message);
    Ok(())
}

pub fn run_vault_list() -> Result<()> {
    let registry = vaults::load_registry()?;
    for (name, entry) in vaults::list_vaults(&registry) {
        let marker = if name == registry.active_vault { "*" } else { " " };
        println!("{marker} {name} — {}", entry.vault_dir);
    }
    Ok(())
}

pub fn run_vault_use(name: &str) -> Result<()> {
    let mut registry = vaults::load_registry()?;
    vaults::use_vault(&mut registry, name.to_string())?;
    vaults::save_registry(&registry)?;
    println!("active vault: {name}");
    Ok(())
}
