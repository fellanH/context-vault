//! `context-vault migrate`: v4 -> v5 (spec: Migration from v4; cards 38/39). Copies exhaust
//! into a data vault, verifies it, builds both indexes, then writes vaults.toml, which
//! switches this machine to v5. Originals and the v4 database are left untouched.
use anyhow::{anyhow, Result};
use cv_index::migrate::{setup, MigrateOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::engine_cmd;

pub async fn run(exhaust: Option<PathBuf>, no_chat_logs: bool) -> Result<()> {
    let (options, targets) = prepare(exhaust, no_chat_logs).await?;
    eprintln!("migrating {} -> {} (a copy; originals stay). Large vaults take minutes.", options.main.display(), options.exhaust.display());
    let report = tokio::task::spawn_blocking(move || setup::setup(&options, &targets)).await??;
    println!("{}", serde_json::to_string_pretty(&report)?);
    eprintln!("done: this machine now runs v5 (registry {}).", report.registry.display());
    Ok(())
}

/// Paths and v4 ids; refuses before any work when the machine is already migrated.
async fn prepare(exhaust: Option<PathBuf>, no_chat_logs: bool) -> Result<(MigrateOptions, setup::Targets)> {
    let registry = unmigrated_registry()?;
    let raw_main = context_vault_core::paths::resolve_vault_dir()?;
    let main = PathBuf::from(&raw_main).canonicalize()?;
    let exhaust = exhaust.unwrap_or_else(|| sibling(&main, "exhaust"));
    std::fs::create_dir_all(&exhaust)?;
    let legacy_ids = legacy_ids(&[raw_main.as_str(), &main.to_string_lossy()]).await?;
    let index_root = registry.parent().ok_or_else(|| anyhow!("registry path has no parent"))?.join("index");
    let options = MigrateOptions { main, exhaust: exhaust.canonicalize()?, chat_logs: !no_chat_logs, legacy_ids };
    Ok((options, setup::Targets { registry, index_root }))
}

/// The registry path, refused when it already lists vaults.
fn unmigrated_registry() -> Result<PathBuf> {
    let registry = cv_core::registry::registry_path()?;
    if !cv_core::Registry::load(&registry)?.vaults.is_empty() {
        anyhow::bail!("{} already lists vaults: this machine is migrated", registry.display());
    }
    Ok(registry)
}

/// `~/omni/vault` -> `~/omni/vault-exhaust`.
fn sibling(main: &Path, suffix: &str) -> PathBuf {
    let name = main.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "vault".into());
    main.with_file_name(format!("{name}-{suffix}"))
}

/// v4's ids by path relative to the vault, so files without an id keep the one agents use.
async fn legacy_ids(prefixes: &[&str]) -> Result<HashMap<String, String>> {
    let engine = engine_cmd::build_sqlite().await?;
    let db = engine.raw_db();
    let conn = db.lock().map_err(|_| anyhow!("v4 database lock poisoned"))?;
    let mut stmt = conn.prepare("SELECT file_path, id FROM vault WHERE file_path IS NOT NULL")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut ids = HashMap::new();
    for (path, id) in rows.flatten() {
        let rel = prefixes.iter().find_map(|p| path.strip_prefix(&format!("{}/", p.trim_end_matches('/'))));
        if let Some(rel) = rel {
            ids.insert(rel.to_string(), id);
        }
    }
    Ok(ids)
}
