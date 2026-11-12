//! Situations on v5 (card 38), in v4's shapes: names by size, and pages in order
//! (situation_order asc, updated desc, id) of about PAGE_CHARS of body text.
//! The vault map is built in engine_map.rs.
use super::V5Backend;
use crate::engine_entry::to_entry;
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use cv_engine::situations::{order_of, situations_of, valid_name, SituationEngine, PAGE_CHARS};
use cv_engine::Entry;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[allow(clippy::double_must_use)]
#[async_trait]
impl SituationEngine for V5Backend {
    async fn list_situations(&self) -> Result<Value> {
        let mut counts: BTreeMap<String, i64> = BTreeMap::new();
        for vault in self.vaults.iter().filter(|v| v.entry.search_by_default) {
            self.catch_up(vault)?;
            for name in vault.index.situation_names()? {
                let live = self.members(&name)?.len() as i64;
                *counts.entry(name).or_default() += live;
            }
        }
        let mut rows: Vec<(String, i64)> = counts.into_iter().filter(|(_, n)| *n > 0).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let rows: Vec<Value> = rows.into_iter().map(|(s, n)| json!({ "situation": s, "entries": n })).collect();
        Ok(json!({ "situations": rows }))
    }

    async fn read_situation(&self, situation: &str, cursor: Option<&str>) -> Result<Value> {
        if !valid_name(situation) {
            bail!("situation \"{situation}\" is not a valid name (lowercase-kebab, max 32 chars)");
        }
        let mut entries = self.members(situation)?;
        entries.sort_by_key(key);
        let after = cursor.filter(|c| !c.is_empty()).map(decode).transpose()?;
        let rest: Vec<&Entry> = entries.iter().filter(|e| after.as_ref().is_none_or(|c| key(e) > *c)).collect();
        let (page, used) = fill(&rest);
        let next_cursor = (used < rest.len()).then(|| encode(&key(rest[used - 1])));
        Ok(json!({ "situation": situation, "entries": page, "next_cursor": next_cursor, "max_chars": PAGE_CHARS }))
    }

    /// Built fresh on every call (bounded reads), so `force` changes nothing.
    async fn vault_map(&self, _force: bool) -> Result<Value> {
        super::map::build(self).await
    }

    async fn list_vaults(&self) -> Result<Value> {
        Ok(json!({ "vaults": self.vault_rows()?, "engine": "v5" }))
    }
}

impl V5Backend {
    /// Live (unexpired) entries carrying `situation`, across the vaults searched by default.
    fn members(&self, situation: &str) -> Result<Vec<Entry>> {
        let now = chrono::Utc::now();
        let mut out = Vec::new();
        for vault in self.vaults.iter().filter(|v| v.entry.search_by_default) {
            for (record, location) in vault.index.situation_members(situation)? {
                let entry = to_entry(record, vault.index.source_path(&location).as_deref(), &vault.taxonomy);
                let expired = entry.expires_at.as_deref().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()).is_some_and(|t| t <= now);
                if !expired {
                    out.push(entry);
                }
            }
        }
        Ok(out)
    }
}

/// Sort and cursor key: order ascending, then newest `updated` first, then id.
type Key = (i64, std::cmp::Reverse<String>, String);

fn key(entry: &Entry) -> Key {
    (order_of(entry.meta.as_ref()), std::cmp::Reverse(entry.updated_at.clone()), entry.id.clone())
}

fn encode(key: &Key) -> String {
    format!("{}|{}|{}", key.0, key.1 .0, key.2)
}

fn decode(cursor: &str) -> Result<Key> {
    let mut parts = cursor.splitn(3, '|');
    let bad = || anyhow!("invalid situation cursor");
    let ord = parts.next().and_then(|p| p.parse().ok()).ok_or_else(bad)?;
    let updated = parts.next().ok_or_else(bad)?.to_string();
    let id = parts.next().ok_or_else(bad)?.to_string();
    Ok((ord, std::cmp::Reverse(updated), id))
}

/// Entries until the page holds PAGE_CHARS of body (at least one; a longer first one is cut).
fn fill(entries: &[&Entry]) -> (Vec<Value>, usize) {
    let (mut page, mut used) = (Vec::new(), 0usize);
    for entry in entries {
        let len = entry.body.chars().count();
        if !page.is_empty() && used + len > PAGE_CHARS {
            break;
        }
        page.push(page_item(entry, len));
        used += len.min(PAGE_CHARS);
    }
    let n = page.len();
    (page, n)
}

fn page_item(entry: &Entry, len: usize) -> Value {
    let truncated = len > PAGE_CHARS;
    let mut item = json!({
        "id": entry.id, "kind": entry.kind, "title": entry.title, "tags": entry.tags,
        "updated_at": entry.updated_at, "situations": situations_of(entry.meta.as_ref()),
        "situation_order": order_of(entry.meta.as_ref()),
        "body": if truncated { entry.body.chars().take(PAGE_CHARS).collect() } else { entry.body.clone() },
    });
    if truncated {
        item["truncated"] = json!(true);
        item["body_length"] = json!(len);
    }
    item
}
