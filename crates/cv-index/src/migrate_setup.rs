//! `context-vault migrate` end to end: copy and verify the exhaust (G3), build both indexes
//! where the engine reads them, and only then write `vaults.toml`, which is what switches
//! the MCP server and CLI to v5. Anything that fails leaves the registry unwritten (v4 stays).
use super::{run, MigrateOptions, MigrateReport};
use crate::{engine::index_dir, UpdateOptions, VaultIndex};
use anyhow::{bail, Context, Result};
use cv_core::{Registry, VaultEntry, VaultType};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct Targets {
    pub registry: PathBuf,
    /// `<data dir>/index`; each vault's index goes in its engine layout below it.
    pub index_root: PathBuf,
}

#[derive(Serialize)]
pub struct SetupReport {
    pub migration: MigrateReport,
    pub main_indexed: u64,
    pub exhaust_indexed: u64,
    pub registry: PathBuf,
}

pub fn setup(options: &MigrateOptions, targets: &Targets) -> Result<SetupReport> {
    if !Registry::load(&targets.registry)?.vaults.is_empty() {
        bail!("{} already lists vaults: this machine is migrated", targets.registry.display());
    }
    let marker = start_marker(&options.exhaust)?;
    let migration = run(options)?;
    if !migration.verify.passed {
        bail!("verification failed (counts, ids or samples); the registry was not written, v4 stays in use");
    }
    let (main_indexed, exhaust_indexed) = build_indexes(options, &targets.index_root)?;
    write_registry(&targets.registry, &options.main, &options.exhaust)?;
    // The machine is migrated now; a marker that won't go away is only clutter.
    let _ = std::fs::remove_file(&marker);
    Ok(SetupReport { migration, main_indexed, exhaust_indexed, registry: targets.registry.clone() })
}

/// A marker says the data vault's segments are this migration's own work, so a failed
/// run can be retried: the next run clears them and starts the copy again.
fn start_marker(exhaust: &Path) -> Result<PathBuf> {
    let marker = exhaust.join(".migrating");
    let segments = exhaust.join("segments");
    if marker.exists() {
        std::fs::remove_dir_all(&segments).or_else(ignore_missing)?;
    } else if segments.exists() {
        bail!("{} already has segments that this migration did not write; choose an empty --exhaust", exhaust.display());
    }
    std::fs::write(&marker, "context-vault migrate in progress; delete with segments/ to start over\n")?;
    Ok(marker)
}

fn build_indexes(options: &MigrateOptions, index_root: &Path) -> Result<(u64, u64)> {
    let full = UpdateOptions { full: true, budget: None, ..Default::default() };
    let exhaust = VaultIndex::open(&options.exhaust, VaultType::Data, &index_dir(index_root, "exhaust"))?.update(&full)?;
    let main = VaultIndex::open(&options.main, VaultType::Knowledge, &index_dir(index_root, "main"))?
        .update(&full)
        .context("knowledge index refused (repair identities first); the registry was not written")?;
    Ok((main.indexed, exhaust.indexed))
}

fn ignore_missing(error: std::io::Error) -> std::io::Result<()> {
    if error.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(error) }
}

/// `main` (knowledge, searched by default) and `exhaust` (data, searched by name only).
pub fn write_registry(path: &Path, main: &Path, exhaust: &Path) -> Result<()> {
    let entry = |kind, dir: &Path, search_by_default| VaultEntry { kind, dir: dir.to_string_lossy().into_owned(), search_by_default };
    let mut registry = Registry { default: "main".into(), ..Default::default() };
    registry.vaults.insert("main".into(), entry(VaultType::Knowledge, main, true));
    registry.vaults.insert("exhaust".into(), entry(VaultType::Data, exhaust, false));
    registry.save(path)
}

#[cfg(test)]
#[path = "migrate_setup_tests.rs"]
mod tests;
