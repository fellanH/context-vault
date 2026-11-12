//! Opening the registry's vaults, and the lookups every engine call shares.
use super::{Vault, V5Backend};
use crate::engine_entry::to_entry;
use crate::{UpdateOptions, VaultIndex};
use anyhow::{anyhow, bail, Result};
use cv_core::{Registry, VaultType};
use cv_engine::Entry;
use cv_vault::VaultTaxonomy;
use std::path::Path;

/// Bumped with every index schema or content change (2: `supersedes`, 3: `situations`,
/// 4: knowledge indexes leave exhaust out).
pub(super) const INDEX_LAYOUT: &str = "layout-4";

impl V5Backend {
    /// The registry at `registry_path()`, with indexes in `<its dir>/index/<name>/`.
    /// `None` when it lists no vaults: the caller keeps v4.
    pub fn from_registry() -> Result<Option<Self>> {
        let path = cv_core::registry::registry_path()?;
        let registry = Registry::load(&path)?;
        if registry.vaults.is_empty() {
            return Ok(None);
        }
        let index_root = path.parent().ok_or_else(|| anyhow!("registry has no parent dir"))?.join("index");
        Self::open(&registry, &index_root).map(Some)
    }

    pub fn open(registry: &Registry, index_root: &Path) -> Result<Self> {
        let (default_name, _) = registry.resolve(None)?;
        let mut vaults = Vec::new();
        for (name, entry) in &registry.vaults {
            let dir = entry.dir_path();
            // A schema change opens a fresh index beside the old one (D2: indexes are derived)
            // instead of refusing to start; `context-vault reindex` fills it.
            let index_dir = super::index_dir(index_root, name);
            let fresh = !index_dir.join("meta.json").exists();
            let index = VaultIndex::open(&dir, entry.kind, &index_dir)?;
            if fresh {
                eprintln!("[context-vault] new v5 index for `{name}`: run `context-vault reindex` to fill it");
            }
            let taxonomy = VaultTaxonomy::load(&dir.to_string_lossy());
            vaults.push(Vault { name: name.clone(), index_dir: index_dir.clone(), entry: entry.clone(), index, taxonomy });
        }
        let default = vaults.iter().position(|v| v.name == default_name).ok_or_else(|| anyhow!("no default vault"))?;
        Ok(Self { vaults, default })
    }

    pub(super) fn knowledge_default(&self) -> Result<&Vault> {
        let vault = &self.vaults[self.default];
        if vault.entry.kind != VaultType::Knowledge {
            bail!("vault `{}` is a data vault: records are appended by collectors, not saved or deleted", vault.name);
        }
        Ok(vault)
    }

    /// The vault holding `id` and the record as an entry.
    pub(super) fn find(&self, id: &str) -> Result<Option<(&Vault, Entry)>> {
        for vault in &self.vaults {
            if let Some((record, location)) = vault.index.get(id)? {
                let file = vault.index.source_path(&location);
                let mut entry = to_entry(record, file.as_deref(), &vault.taxonomy);
                if entry.superseded_by.is_none() {
                    entry.superseded_by = self.superseder(id)?;
                }
                return Ok(Some((vault, entry)));
            }
        }
        Ok(None)
    }

    /// One row per registered vault, for `list_vaults` (spec: Surface).
    pub(super) fn vault_rows(&self) -> Result<Vec<serde_json::Value>> {
        let mut rows = Vec::new();
        for (i, vault) in self.vaults.iter().enumerate() {
            self.catch_up(vault)?;
            let source = vault.index.plan()?;
            rows.push(serde_json::json!({
                "source_files": source.files,
                "source_bytes": source.bytes,
                "name": vault.name,
                "type": vault.entry.kind,
                "dir": vault.index.root(),
                "records": vault.index.count()?,
                "index_bytes": vault.index.size_bytes()?,
                "default": i == self.default,
                "search_by_default": vault.entry.search_by_default,
            }));
        }
        Ok(rows)
    }

    /// A record in any vault that lists `id` in its `supersedes`.
    pub(super) fn superseder(&self, id: &str) -> Result<Option<String>> {
        for vault in &self.vaults {
            if let Some(newer) = vault.index.superseded_by(id)? {
                return Ok(Some(newer));
            }
        }
        Ok(None)
    }

    /// D4: index what changed since the checkpoint, within the default 200 ms; a busy lock is fine.
    pub(super) fn catch_up(&self, vault: &Vault) -> Result<()> {
        vault.index.update(&UpdateOptions::default()).map(|_| ())
    }
}
