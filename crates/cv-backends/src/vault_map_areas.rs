//! Turning the map's rows into its JSON: key entries per kind, areas, top tags, one-line summaries.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use crate::vault_map_query::{Group, Row};

pub const KEY_ENTRIES_PER_GROUP: usize = 8;
pub const TOP_TAGS: usize = 15;
const SUMMARY_CHARS: usize = 140;
const TITLE_CHARS: usize = 120;

type Situated = HashMap<String, Vec<String>>;

/// The body without fenced code blocks and markdown marks, on one line.
fn strip_fences(text: &str) -> String {
    let mut in_fence = false;
    let mut kept: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence {
            kept.push(line.trim_start().trim_start_matches(['#', '>', '-', '*']).trim());
        }
    }
    let joined = kept.join(" ").replace(['*', '_', '`'], "");
    joined.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn first_sentence(flat: &str) -> String {
    let chars: Vec<char> = flat.chars().collect();
    let ends = |i: usize| matches!(chars[i], '.' | '!' | '?') && chars.get(i + 1).is_none_or(|c| c.is_whitespace());
    let end = (0..chars.len()).find(|&i| ends(i)).map_or(chars.len(), |e| e + 1);
    chars[..end].iter().collect()
}

/// First sentence of a body, markdown noise removed, at most 140 characters.
pub fn summarize(text: &str) -> String {
    let sentence = first_sentence(&strip_fences(text));
    if sentence.chars().count() <= SUMMARY_CHARS {
        return sentence;
    }
    format!("{}…", sentence.chars().take(SUMMARY_CHARS - 1).collect::<String>().trim_end())
}

fn key_entry(row: &Row, situated: Option<&Vec<String>>) -> Value {
    let summary = summarize(&row.head);
    let title = row.title.as_deref().unwrap_or("").trim();
    let fallback = if summary.is_empty() { format!("({})", row.kind) } else { summary.clone() };
    let shown = if title.is_empty() { fallback } else { title.to_string() };
    let mut v = json!({
        "id": row.id,
        "title": shown.chars().take(TITLE_CHARS).collect::<String>(),
        "summary": summary,
        "updated_at": row.updated,
    });
    if let Some(s) = situated {
        v["situations"] = json!(s);
    }
    v
}

fn row_tags(row: &Row) -> HashSet<String> {
    let tags: Vec<String> = row.tags.as_deref().and_then(|t| serde_json::from_str(t).ok()).unwrap_or_default();
    tags.into_iter().filter(|t| !t.is_empty()).collect()
}

/// Top 15 tags with counts over the given rows.
pub fn top_tags(rows: &[Row]) -> Vec<Value> {
    let mut counts: HashMap<String, i64> = HashMap::new();
    for tag in rows.iter().flat_map(row_tags) {
        *counts.entry(tag).or_default() += 1;
    }
    let mut all: Vec<(String, i64)> = counts.into_iter().collect();
    all.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    all.into_iter().take(TOP_TAGS).map(|(tag, entries)| json!({ "tag": tag, "entries": entries })).collect()
}

/// Entries with a situation first, then the most recently updated.
fn rank(a: &Row, b: &Row, situated: &Situated) -> std::cmp::Ordering {
    situated
        .contains_key(&b.id)
        .cmp(&situated.contains_key(&a.id))
        .then_with(|| b.updated.cmp(&a.updated))
        .then_with(|| a.id.cmp(&b.id))
}

fn kind_value(group: &Group, mut candidates: Vec<&Row>, situated: &Situated) -> Value {
    candidates.sort_by(|a, b| rank(a, b, situated));
    let keys: Vec<Value> = candidates.iter().take(KEY_ENTRIES_PER_GROUP).map(|r| key_entry(r, situated.get(&r.id))).collect();
    json!({ "kind": group.kind, "entries": group.entries, "key_entries": keys })
}

fn add_to_area(areas: &mut Vec<(String, i64, Vec<Value>)>, group: &Group, kind: Value) {
    match areas.iter_mut().find(|a| a.0 == group.category) {
        Some(area) => {
            area.1 += group.entries;
            area.2.push(kind);
        }
        None => areas.push((group.category.clone(), group.entries, vec![kind])),
    }
}

/// Areas (category > kind) with up to 8 key entries per kind, biggest area first.
pub fn build_areas(groups: &[Group], pool: &HashMap<String, Row>, situated: &Situated) -> Vec<Value> {
    let mut by_group: HashMap<(&str, &str), Vec<&Row>> = HashMap::new();
    for row in pool.values() {
        by_group.entry((row.category.as_str(), row.kind.as_str())).or_default().push(row);
    }
    let mut areas: Vec<(String, i64, Vec<Value>)> = Vec::new();
    for group in groups {
        let candidates = by_group.remove(&(group.category.as_str(), group.kind.as_str())).unwrap_or_default();
        add_to_area(&mut areas, group, kind_value(group, candidates, situated));
    }
    areas.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    areas.into_iter().map(|(category, entries, kinds)| json!({ "category": category, "entries": entries, "kinds": kinds })).collect()
}
