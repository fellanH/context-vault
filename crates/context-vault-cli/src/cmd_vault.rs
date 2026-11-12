//! `context-vault vault ...`: on v5 the verbs work on vaults.toml (spec: Registry); before a
//! migration they keep v4's vaults.json behaviour. Adding to an empty registry is refused:
//! that would switch the machine to v5 without the migration's checks.
use anyhow::{bail, Result};
use cv_core::{Registry, VaultEntry, VaultType};
use std::path::PathBuf;

use crate::cli::VaultCommands;
use crate::engine_cmd;

pub fn run(command: VaultCommands) -> Result<()> {
    match command {
        VaultCommands::List => list(),
        VaultCommands::Use { name } => use_vault(&name),
        VaultCommands::Add { name, dir, data, by_name_only } => add(&name, dir, data, by_name_only),
        VaultCommands::Remove { name } => remove(&name),
    }
}

/// v5 `status`: per vault, records, what the sources and the index take on disk (card 34).
/// v4 has no registry and prints nothing here.
pub async fn print_sizes(engine: &dyn context_vault_core::engine::VaultEngine) -> Result<()> {
    let vaults = engine.list_vaults().await?;
    let mb = |v: &serde_json::Value| v.as_f64().unwrap_or(0.0) / 1e6;
    for v in vaults["vaults"].as_array().into_iter().flatten() {
        println!(
            "vault {}: {} records, {} source files ({:.1} MB), index {:.1} MB",
            v["name"].as_str().unwrap_or(""), v["records"], v["source_files"], mb(&v["source_bytes"]), mb(&v["index_bytes"])
        );
    }
    Ok(())
}

fn load() -> Result<(PathBuf, Registry)> {
    let path = cv_core::registry::registry_path()?;
    let registry = Registry::load(&path)?;
    Ok((path, registry))
}

fn list() -> Result<()> {
    let (_, registry) = load()?;
    if registry.vaults.is_empty() {
        return engine_cmd::run_vault_list();
    }
    for (name, entry) in &registry.vaults {
        println!("{}", row(name, entry, *name == registry.default));
    }
    Ok(())
}

fn row(name: &str, entry: &VaultEntry, default: bool) -> String {
    let marker = if default { "*" } else { " " };
    let searched = if entry.search_by_default { "searched by default" } else { "searched by name" };
    format!("{marker} {name} — {:?}, {}, {searched}", entry.kind, entry.dir)
}

fn add(name: &str, dir: PathBuf, data: bool, by_name_only: bool) -> Result<()> {
    let (path, mut registry) = load()?;
    if registry.vaults.is_empty() {
        bail!("no v5 registry yet: run `context-vault migrate` first");
    }
    if registry.vaults.contains_key(name) {
        bail!("a vault named `{name}` exists; remove it first");
    }
    std::fs::create_dir_all(&dir)?;
    let kind = if data { VaultType::Data } else { VaultType::Knowledge };
    let entry = VaultEntry { kind, dir: dir.canonicalize()?.to_string_lossy().into_owned(), search_by_default: !by_name_only };
    registry.vaults.insert(name.to_string(), entry);
    registry.save(&path)?;
    println!("added vault `{name}`; run `context-vault reindex` to index it");
    Ok(())
}

/// Forgets the vault; its files and index stay on disk.
fn remove(name: &str) -> Result<()> {
    let (path, mut registry) = load()?;
    if name == registry.default {
        bail!("`{name}` is the default vault; make another the default first (`vault use`)");
    }
    if registry.vaults.remove(name).is_none() {
        bail!("no vault named `{name}`");
    }
    registry.save(&path)?;
    println!("removed vault `{name}` from the registry (files untouched)");
    Ok(())
}

fn use_vault(name: &str) -> Result<()> {
    let (path, mut registry) = load()?;
    if registry.vaults.is_empty() {
        return engine_cmd::run_vault_use(name);
    }
    if !registry.vaults.contains_key(name) {
        bail!("no vault named `{name}`");
    }
    registry.default = name.to_string();
    registry.save(&path)?;
    println!("default vault: {name}");
    Ok(())
}
