//! `vaults.toml`: every vault this machine knows, by name. Replaces v4's `vaults.json`
//! (never read by the engine), the `config.json` paths and the env-var pairs.
//!
//! ```toml
//! default = "main"
//! [vaults.main]
//! type = "knowledge"
//! dir = "~/omni/vault"
//! [vaults.exhaust]
//! type = "data"
//! dir = "~/omni/vault-exhaust"
//! search_by_default = false
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

pub const REGISTRY_FILENAME: &str = "vaults.toml";
/// Selects a vault by name for one process, overriding `default`.
pub const VAULT_ENV: &str = "CONTEXT_VAULT";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VaultType {
    /// One markdown file per record; human-scale, edited by people.
    Knowledge,
    /// Append-only JSONL segments; machine-scale.
    Data,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultEntry {
    #[serde(rename = "type")]
    pub kind: VaultType,
    pub dir: String,
    #[serde(default = "default_true")]
    pub search_by_default: bool,
}

fn default_true() -> bool {
    true
}

impl VaultEntry {
    /// `dir` with a leading `~/` expanded.
    pub fn dir_path(&self) -> PathBuf {
        expand_tilde(&self.dir)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub vaults: BTreeMap<String, VaultEntry>,
}

impl Registry {
    /// Load and validate. A missing file is an empty registry; an invalid one is an error,
    /// never a silent fallback (a wrong vault is worse than no vault).
    pub fn load(path: &Path) -> Result<Self> {
        let raw = match std::fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e).with_context(|| format!("cannot read {}", path.display())),
        };
        let reg: Self = toml::from_str(&raw).with_context(|| format!("invalid {}", path.display()))?;
        reg.validate()?;
        Ok(reg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, toml::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Refuses: a default that names no vault, bad names, and one vault's dir inside
    /// another's (the same files would be indexed twice).
    pub fn validate(&self) -> Result<()> {
        if !self.vaults.is_empty() && !self.vaults.contains_key(&self.default) {
            bail!("default vault `{}` is not in the registry", self.default);
        }
        if let Some(name) = self.vaults.keys().find(|n| !valid_name(n)) {
            bail!("vault name `{name}`: use letters, digits, `-` or `_`");
        }
        self.check_nesting()
    }

    fn check_nesting(&self) -> Result<()> {
        let dirs: Vec<(&String, PathBuf)> = self.vaults.iter().map(|(n, v)| (n, v.dir_path())).collect();
        for (a, da) in &dirs {
            if let Some((b, _)) = dirs.iter().find(|(b, db)| b != a && da.starts_with(db)) {
                bail!("vault `{a}` lies inside vault `{b}`; nested vaults would index the same files twice");
            }
        }
        Ok(())
    }

    /// The vault to use: `name` if given, else `$CONTEXT_VAULT`, else `default`.
    pub fn resolve(&self, name: Option<&str>) -> Result<(String, VaultEntry)> {
        let env = std::env::var(VAULT_ENV).ok();
        let chosen = name.map(str::to_string).or(env).unwrap_or_else(|| self.default.clone());
        let entry = self.vaults.get(&chosen).ok_or_else(|| {
            let known: Vec<&str> = self.vaults.keys().map(String::as_str).collect();
            anyhow!("no vault named `{chosen}` (known: {})", known.join(", "))
        })?;
        Ok((chosen, entry.clone()))
    }
}

/// `$CONTEXT_MCP_DATA_DIR/vaults.toml`, else `~/.context-mcp/vaults.toml`.
pub fn registry_path() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("CONTEXT_MCP_DATA_DIR") {
        return Ok(PathBuf::from(dir).join(REGISTRY_FILENAME));
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home directory"))?;
    Ok(home.join(".context-mcp").join(REGISTRY_FILENAME))
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn expand_tilde(p: &str) -> PathBuf {
    match (p.strip_prefix("~/"), dirs::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(p),
    }
}
