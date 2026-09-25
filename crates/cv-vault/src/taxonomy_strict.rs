//! The write path's vault.toml loader. Fails closed where `load()` falls back.

use std::path::Path;

use super::{TaxonomyFile, VaultTaxonomy, VAULT_TOML_FILENAME};

/// Keys vault.toml may carry, per table. Anything else is a typo the lenient
/// `load()` ignores (serde default) but `load_strict()` must not: a misspelt
/// `personal_fre` or `feilds` would silently switch a refuser off. `[vault]` and
/// field specs are covered by `deny_unknown_fields`; this covers the tables that
/// stay lenient for `load()`'s sake.
const TOP_LEVEL_KEYS: &[&str] = &["schema_version", "vault", "default", "kinds"];
const KIND_KEYS: &[&str] = &["category", "tier", "identity", "folder", "fields"];

impl VaultTaxonomy {
    /// Load for the write path: like `load()`, but a vault.toml that exists and fails
    /// to parse is an error, not a fallback. The lenient fallback is right for reads
    /// (a typo must not break search) and wrong for writes once the file can declare
    /// refusers: falling back to `minimal()` would drop every schema and policy and
    /// accept the very saves they exist to refuse.
    pub fn load_strict(vault_dir: &str) -> anyhow::Result<Self> {
        let path = Path::new(vault_dir).join(VAULT_TOML_FILENAME);
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::minimal()),
            Err(e) => anyhow::bail!("cannot read {}: {e}", path.display()),
        };
        let file = toml::from_str::<TaxonomyFile>(&raw)
            .map_err(|e| anyhow::anyhow!("invalid {} — save refused: {e}", path.display()))?;
        let taxonomy = Self::from_file(file);
        let unknown = unknown_keys(&raw);
        if !unknown.is_empty() {
            let msg = format!("unknown key(s) in {}: {}", path.display(), unknown.join(", "));
            if taxonomy.declares_refusers() {
                anyhow::bail!("{msg} — save refused (a misspelt key can switch a refuser off)");
            }
            eprintln!("warning: {msg}");
        }
        Ok(taxonomy)
    }

    /// Whether this vault.toml declares anything that refuses saves.
    pub fn declares_refusers(&self) -> bool {
        !self.policy.is_default()
            || !self.default.fields.is_empty()
            || self.kinds.values().any(|r| !r.fields.is_empty())
    }
}

/// Keys outside the known vault.toml shape, as dotted paths.
fn unknown_keys(raw: &str) -> Vec<String> {
    let Ok(toml::Value::Table(top)) = raw.parse::<toml::Value>() else {
        return Vec::new();
    };
    let mut unknown: Vec<String> = top
        .keys()
        .filter(|k| !TOP_LEVEL_KEYS.contains(&k.as_str()))
        .cloned()
        .collect();
    let rules = top.get("default").map(|r| ("default".to_string(), r)).into_iter().chain(
        top.get("kinds")
            .and_then(|k| k.as_table())
            .into_iter()
            .flatten()
            .map(|(name, r)| (format!("kinds.{name}"), r)),
    );
    for (path, rule) in rules {
        let Some(t) = rule.as_table() else { continue };
        unknown.extend(
            t.keys()
                .filter(|k| !KIND_KEYS.contains(&k.as_str()))
                .map(|k| format!("{path}.{k}")),
        );
    }
    unknown
}

#[cfg(test)]
#[path = "taxonomy_strict_tests.rs"]
mod tests;
