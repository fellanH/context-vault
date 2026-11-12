//! `schema init` / `schema show` — inspect and scaffold the per-vault taxonomy
//! (kind -> category/tier/identity/folder, plus the refusers: per-kind `fields` and the
//! `[vault]` policy), see specs/vault-taxonomy.md.

use std::path::Path;

use anyhow::Result;
use context_vault_core::engine::ReindexOptions;
use context_vault_core::paths::resolve_vault_dir;
use cv_vault::VaultTaxonomy;

use crate::engine_cmd;

pub fn run_schema_init() -> Result<()> {
    let vault_dir = resolve_vault_dir()?;
    let path = Path::new(&vault_dir).join("vault.toml");

    if path.exists() {
        println!("{} already exists — not overwriting.", path.display());
        return Ok(());
    }

    std::fs::write(&path, VaultTaxonomy::minimal().to_toml_string())?;
    println!("wrote {}", path.display());
    println!("This is a blank starting point (flat per-kind folders, no ontology assumed).");
    println!("Add [kinds.<name>] entries to define your own categories, tiers, and folders.");
    Ok(())
}

pub fn run_schema_show() -> Result<()> {
    let vault_dir = resolve_vault_dir()?;
    let path = Path::new(&vault_dir).join("vault.toml");
    // Strict: show what saves will actually be checked against. A file the write
    // path would refuse is reported as such, not papered over with the fallback.
    let taxonomy = VaultTaxonomy::load_strict(&vault_dir)?;

    if path.exists() {
        println!("# from {}\n", path.display());
    } else {
        println!("# no vault.toml found at {} — showing the zero-config default (flat per-kind folders)\n", vault_dir);
    }

    println!("{}", taxonomy.to_toml_string());
    Ok(())
}

pub async fn run_schema_apply(dry_run: bool) -> Result<()> {
    let vault_dir = resolve_vault_dir()?;
    let summary = cv_vault::plan_reorg(&vault_dir, dry_run)?;
    let verb = if dry_run { "would move" } else { "moved" };

    for (old, new) in &summary.moved {
        println!("{verb}: {old} -> {new}");
    }
    for path in &summary.skipped {
        println!("skip (destination already exists, resolve manually): {path}");
    }
    println!(
        "{} {verb}, {} skipped, {} already correctly placed",
        summary.moved.len(),
        summary.skipped.len(),
        summary.unchanged
    );

    if dry_run {
        println!("dry run: nothing was written. Re-run without --dry-run to apply.");
        return Ok(());
    }

    if !summary.moved.is_empty() {
        println!("reindexing to sync file_path + category...");
        let engine = engine_cmd::build_engine().await?;
        let result = engine.reindex(ReindexOptions { full: false }).await?;
        println!(
            "reindexed {} entries in {:.0}ms",
            result.entries_reindexed, result.duration_ms
        );
    }

    Ok(())
}
