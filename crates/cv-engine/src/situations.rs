//! Situations: an entry can say when it matters (`fresh`, `post-compaction`, `before-deploy`, ...).
//! They live in the entry's meta (`situations`: array, `situation_order`: integer), so they ride
//! in the markdown frontmatter and survive a reindex; the backend keeps an index table over them.

use async_trait::async_trait;
use serde_json::Value;

pub const MAX_SITUATIONS: usize = 6;
pub const MAX_NAME_CHARS: usize = 32;
pub const DEFAULT_ORDER: i64 = 100;
pub const MAX_ORDER: i64 = 100_000;
/// Body characters per `context_for` page (at least one entry; a longer single body is cut).
pub const PAGE_CHARS: usize = 12_000;

/// Reads over the situation index and the vault map. A supertrait of `VaultEngine`.
// async_trait adds must_use to its boxed Future, which is already must_use.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait SituationEngine {
    /// `{situations: [{situation, entries}]}` over live entries, biggest first.
    async fn list_situations(&self) -> anyhow::Result<Value>;
    /// One page of a situation's entries with bodies, plus `next_cursor`.
    async fn read_situation(&self, situation: &str, cursor: Option<&str>) -> anyhow::Result<Value>;
    /// The generated vault overview; `force` regenerates it (rate limited to once a minute).
    async fn vault_map(&self, force: bool) -> anyhow::Result<Value>;
    /// `{vaults: [{name, type, dir, records, index_bytes, default, search_by_default}]}`.
    /// v4 has one vault and no registry; v5 lists `vaults.toml`.
    async fn list_vaults(&self) -> anyhow::Result<Value> {
        Ok(serde_json::json!({ "vaults": [], "engine": "v4", "note": "one vault, no registry: see context_status" }))
    }
}

/// Lowercase-kebab: `^[a-z0-9]+(-[a-z0-9]+)*$`, at most 32 chars.
pub fn valid_name(name: &str) -> bool {
    let part_ok = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    name.chars().count() <= MAX_NAME_CHARS && name.split('-').all(part_ok)
}

fn one_name(item: &Value) -> Result<&str, String> {
    let name = item.as_str().ok_or("situations must be an array of strings")?;
    if valid_name(name) {
        return Ok(name);
    }
    Err(format!("situation \"{name}\" is not lowercase-kebab (a-z, 0-9, hyphens; max {MAX_NAME_CHARS} chars)"))
}

/// Validate a `situations` value: array of kebab names, max 6, duplicates dropped.
pub fn normalize_situations(value: &Value) -> Result<Vec<String>, String> {
    let items = value.as_array().ok_or("situations must be an array of strings")?;
    let mut out: Vec<String> = Vec::new();
    for item in items {
        let name = one_name(item)?;
        if !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    if out.len() > MAX_SITUATIONS {
        return Err(format!("at most {MAX_SITUATIONS} situations per entry"));
    }
    Ok(out)
}

pub fn normalize_order(value: &Value) -> Result<i64, String> {
    match value.as_i64() {
        Some(n) if (0..=MAX_ORDER).contains(&n) => Ok(n),
        _ => Err(format!("situation_order must be an integer 0..{MAX_ORDER}")),
    }
}

/// The situations stored in an entry's meta (empty when none).
pub fn situations_of(meta: Option<&Value>) -> Vec<String> {
    let list = meta.and_then(|m| m.get("situations")).and_then(|v| v.as_array());
    list.map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default()
}

pub fn order_of(meta: Option<&Value>) -> i64 {
    meta.and_then(|m| m.get("situation_order")).and_then(|v| v.as_i64()).unwrap_or(DEFAULT_ORDER)
}
