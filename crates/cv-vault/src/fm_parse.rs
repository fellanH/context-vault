//! Frontmatter parsing + per-kind markdown extraction. Emission lives in
//! `frontmatter.rs`. Faithful port of the TS `parseFrontmatter` /
//! `extractCustomMeta` / `parseEntryFromMarkdown`.

use regex::Regex;
use serde_json::{Map, Value};
use std::collections::HashSet;

pub fn parse_frontmatter(text: &str) -> (Map<String, Value>, String) {
    let normalized = text.replace("\r\n", "\n");
    let re = Regex::new(r"^---\n([\s\S]*?)\n---\n?([\s\S]*)$").unwrap();
    let caps = match re.captures(&normalized) {
        Some(c) => c,
        None => return (Map::new(), normalized.trim().to_string()),
    };

    let body = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim().to_string();
    let lines: Vec<&str> = caps[1].split('\n').collect();
    let mut meta = Map::new();
    let mut i = 0;
    while i < lines.len() {
        i = parse_fm_line(&lines, i, &mut meta);
    }
    (meta, body)
}

/// Parse one frontmatter line into `meta`, returning the index of the next line to
/// process (multiline `tags:` lists consume several lines).
fn parse_fm_line(lines: &[&str], i: usize, meta: &mut Map<String, Value>) -> usize {
    let line = lines[i];
    let idx = match line.find(':') {
        Some(x) => x,
        None => return i + 1,
    };
    let key = line[..idx].trim();
    let val_str = line[idx + 1..].trim();

    if key == "tags" && val_str.is_empty() {
        let (val, last_consumed) = parse_multiline_tags(lines, i);
        meta.insert(key.to_string(), val);
        return last_consumed + 1;
    }
    meta.insert(key.to_string(), parse_fm_value(key, val_str));
    i + 1
}

/// A YAML block list under `tags:`. Returns the parsed value and the index of the
/// last line consumed (the caller advances past it).
fn parse_multiline_tags(lines: &[&str], start_idx: usize) -> (Value, usize) {
    let dash_re = Regex::new(r"^\s*-\s+(.+)$").unwrap();
    let mut items = Vec::new();
    let mut next_idx = start_idx + 1;
    while next_idx < lines.len() {
        let caps = match dash_re.captures(lines[next_idx]) {
            Some(c) => c,
            None => break,
        };
        let item = caps[1].trim().trim_matches(|c| c == '"' || c == '\'');
        items.push(Value::String(item.to_string()));
        next_idx += 1;
    }
    if items.is_empty() {
        (Value::String(String::new()), start_idx)
    } else {
        (Value::Array(items), next_idx - 1)
    }
}

fn parse_fm_value(key: &str, val_str: &str) -> Value {
    if let Some(v) = try_parse_quoted(val_str) {
        return v;
    }
    if let Some(v) = try_parse_array(val_str) {
        return v;
    }
    // Preserve reserved identifiers/dates as strings for existing v4 files.
    // Custom typed fields use JSON scalars and inline objects, as emitted below.
    if !RESERVED_FM_KEYS.contains(&key) {
        if let Ok(value) = serde_json::from_str::<Value>(val_str) {
            return value;
        }
    }
    Value::String(val_str.to_string())
}

fn try_parse_quoted(s: &str) -> Option<Value> {
    if s.len() < 2 || !s.starts_with('"') || !s.ends_with('"') || s.starts_with("[\"") {
        return None;
    }
    serde_json::from_str(s).ok()
}

fn try_parse_array(s: &str) -> Option<Value> {
    if !s.starts_with('[') || !s.ends_with(']') {
        return None;
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(s) {
        return Some(parsed);
    }
    let items: Vec<Value> = s[1..s.len() - 1]
        .split(',')
        .map(|part| Value::String(part.trim().trim_matches(|c| c == '"' || c == '\'').to_string()))
        .collect();
    Some(Value::Array(items))
}

const RESERVED_FM_KEYS: &[&str] = &[
    "id", "title", "kind", "tier", "tags", "source", "created", "updated", "identity_key",
    "expires_at", "supersedes", "related_to",
];

pub fn extract_custom_meta(fm_meta: &Map<String, Value>) -> Option<Map<String, Value>> {
    let reserved: HashSet<&str> = RESERVED_FM_KEYS.iter().copied().collect();
    let mut custom = Map::new();
    for (k, v) in fm_meta {
        if !reserved.contains(k.as_str()) {
            custom.insert(k.clone(), v.clone());
        }
    }
    if custom.is_empty() {
        None
    } else {
        Some(custom)
    }
}

#[cfg(test)]
#[path = "fm_parse_tests.rs"]
mod tests;
