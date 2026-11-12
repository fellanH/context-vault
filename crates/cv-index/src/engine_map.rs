//! The vault map on v5 (card 38), in v4's shape: areas (category > kind) with counts and up
//! to 8 key entries each, top tags, situations, totals. Built from the index on each call,
//! with bounded reads (16 recent entries for each of the 24 biggest kinds, 400 for tags);
//! no cache, since there is no database to keep it in. The text helpers follow
//! cv-backends/vault_map_areas.rs, which card 41 removes.
use super::{Vault, V5Backend};
use crate::engine_entry::to_entry;
use anyhow::Result;
use cv_engine::situations::{situations_of, SituationEngine};
use cv_engine::Entry;
use serde_json::{json, Value};
use std::collections::HashMap;

const MAX_GROUPS: usize = 60;
const MAX_GROUPS_WITH_ENTRIES: usize = 24;
const PER_GROUP_CANDIDATES: usize = 16;
const KEY_ENTRIES_PER_GROUP: usize = 8;
const RECENT_SAMPLE: usize = 400;
const TOP_TAGS: usize = 15;
const SUMMARY_CHARS: usize = 140;
const TITLE_CHARS: usize = 120;

struct Group<'a> {
    vault: &'a Vault,
    category: String,
    kind: String,
    entries: u64,
}

pub(super) async fn build(engine: &V5Backend) -> Result<Value> {
    let mut all = Vec::new();
    let mut recent = Vec::new();
    for vault in engine.vaults.iter().filter(|v| v.entry.search_by_default) {
        engine.catch_up(vault)?;
        for (kind, entries) in vault.index.kind_counts()? {
            all.push(Group { vault, category: vault.taxonomy.category_for(&kind).to_string(), kind, entries });
        }
        recent.extend(entries_of(vault, None, RECENT_SAMPLE)?);
    }
    all.sort_by(|a, b| b.entries.cmp(&a.entries).then_with(|| a.kind.cmp(&b.kind)));
    let shown = &all[..all.len().min(MAX_GROUPS)];
    let mut areas: Vec<(String, u64, Vec<Value>)> = Vec::new();
    for (i, group) in shown.iter().enumerate() {
        let keys = if i < MAX_GROUPS_WITH_ENTRIES { key_entries(group)? } else { Vec::new() };
        let kind = json!({ "kind": group.kind, "entries": group.entries, "key_entries": keys });
        match areas.iter_mut().find(|a| a.0 == group.category) {
            Some(area) => {
                area.1 += group.entries;
                area.2.push(kind);
            }
            None => areas.push((group.category.clone(), group.entries, vec![kind])),
        }
    }
    areas.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Ok(json!({
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "totals": { "live": all.iter().map(|g| g.entries).sum::<u64>(), "groups": all.len() },
        "areas": areas.into_iter().map(|(category, entries, kinds)| json!({ "category": category, "entries": entries, "kinds": kinds })).collect::<Vec<_>>(),
        "other_groups": all.len() - shown.len(),
        "tags": top_tags(&recent),
        "tags_sampled_from": recent.len(),
        "situations": engine.list_situations().await?["situations"].clone(),
        "regenerated": true,
    }))
}

fn entries_of(vault: &Vault, kind: Option<&str>, limit: usize) -> Result<Vec<Entry>> {
    let records = vault.index.recent(kind, limit, 0)?;
    Ok(records.into_iter().map(|(record, location)| to_entry(record, vault.index.source_path(&location).as_deref(), &vault.taxonomy)).collect())
}

/// Entries with a situation first, then the most recently updated, then id.
fn key_entries(group: &Group) -> Result<Vec<Value>> {
    let mut candidates = entries_of(group.vault, Some(&group.kind), PER_GROUP_CANDIDATES)?;
    let situated = |e: &Entry| !situations_of(e.meta.as_ref()).is_empty();
    candidates.sort_by(|a, b| situated(b).cmp(&situated(a)).then_with(|| b.updated_at.cmp(&a.updated_at)).then_with(|| a.id.cmp(&b.id)));
    Ok(candidates.iter().take(KEY_ENTRIES_PER_GROUP).map(key_entry).collect())
}

fn key_entry(entry: &Entry) -> Value {
    let summary = summarize(&entry.body.chars().take(400).collect::<String>());
    let title = entry.title.as_deref().unwrap_or("").trim();
    let fallback = if summary.is_empty() { format!("({})", entry.kind) } else { summary.clone() };
    let shown = if title.is_empty() { fallback } else { title.to_string() };
    let mut v = json!({ "id": entry.id, "title": shown.chars().take(TITLE_CHARS).collect::<String>(), "summary": summary, "updated_at": entry.updated_at });
    let situations = situations_of(entry.meta.as_ref());
    if !situations.is_empty() {
        v["situations"] = json!(situations);
    }
    v
}

fn top_tags(entries: &[Entry]) -> Vec<Value> {
    let mut counts: HashMap<&str, i64> = HashMap::new();
    for entry in entries {
        let mut seen: Vec<&str> = entry.tags.iter().map(String::as_str).filter(|t| !t.is_empty()).collect();
        seen.sort_unstable();
        seen.dedup();
        for tag in seen {
            *counts.entry(tag).or_default() += 1;
        }
    }
    let mut all: Vec<(&str, i64)> = counts.into_iter().collect();
    all.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    all.into_iter().take(TOP_TAGS).map(|(tag, entries)| json!({ "tag": tag, "entries": entries })).collect()
}

/// First sentence of a body, markdown noise and fenced code removed, at most 140 characters.
fn summarize(text: &str) -> String {
    let mut in_fence = false;
    let mut kept: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence {
            kept.push(line.trim_start().trim_start_matches(['#', '>', '-', '*']).trim());
        }
    }
    let flat = kept.join(" ").replace(['*', '_', '`'], "").split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = flat.chars().collect();
    let ends = |i: usize| matches!(chars[i], '.' | '!' | '?') && chars.get(i + 1).is_none_or(|c| c.is_whitespace());
    let end = (0..chars.len()).find(|&i| ends(i)).map_or(chars.len(), |e| e + 1);
    let sentence: String = chars[..end].iter().collect();
    if sentence.chars().count() <= SUMMARY_CHARS {
        return sentence;
    }
    format!("{}…", sentence.chars().take(SUMMARY_CHARS - 1).collect::<String>().trim_end())
}
