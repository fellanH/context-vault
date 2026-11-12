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

/// v5 when `vaults.toml` lists vaults (card 38), else v4's SQLite engine as before.
pub async fn build_engine() -> Result<Arc<dyn VaultEngine>> {
    if let Some(v5) = cv_index::engine::V5Backend::from_registry()? {
        return Ok(Arc::new(v5));
    }
    Ok(Arc::new(build_sqlite().await?))
}

/// v4's engine, for commands that edit its database directly (archive, tiers, move).
/// Refused on a migrated vault: v5 keeps no database, so those edits would go nowhere.
pub async fn build_sqlite() -> Result<SqliteBackend> {
    if cv_index::engine::V5Backend::from_registry()?.is_some() {
        anyhow::bail!("this command edits v4's database; this vault runs on v5 (vaults.toml), which has none");
    }
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
    print_paths()?;
    crate::cmd_vault::print_sizes(engine.as_ref()).await?;
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

/// On v5 the health line names the vaults; v4's db and vault paths would mislead.
fn print_paths() -> Result<()> {
    let registry = cv_core::registry::registry_path()?;
    if cv_core::Registry::load(&registry)?.vaults.is_empty() {
        println!("db:        {}", resolve_db_path()?);
        println!("vault dir: {}", resolve_vault_dir()?);
    } else {
        println!("registry:  {}", registry.display());
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
    crate::cmd_collectors::print_collectors(&vault, &db)
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
