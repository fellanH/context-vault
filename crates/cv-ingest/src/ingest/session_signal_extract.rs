//! Low-level per-block/per-turn extraction helpers for `session_transcript`: split
//! out purely to stay under this repo's per-file function-count ceiling.

use serde_json::Value;
use std::collections::HashSet;

pub(super) fn is_role(turn: &Value, role: &str) -> bool {
    turn.get("role").and_then(|r| r.as_str()) == Some(role)
}

pub(super) fn tool_use_blocks(turns: &[Value]) -> Vec<&Value> {
    turns
        .iter()
        .flat_map(|t| t.get("content").and_then(|c| c.as_array()).into_iter().flatten())
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
        .collect()
}

fn block_input_str(block: &Value, keys: &[&str]) -> Option<String> {
    let input = block.get("input")?;
    keys.iter()
        .find_map(|k| input.get(k).and_then(|v| v.as_str()))
        .map(|s| s.to_string())
}

pub(super) fn files_modified(blocks: &[&Value]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for block in blocks {
        let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let is_edit = name == "Write" || name == "Edit";
        let Some(path) = is_edit.then(|| block_input_str(block, &["file_path", "path"])).flatten() else {
            continue;
        };
        if seen.insert(path.clone()) {
            out.push(path);
        }
    }
    out
}

pub(super) fn commands_run(blocks: &[&Value]) -> Vec<String> {
    blocks
        .iter()
        .filter(|b| b.get("name").and_then(|n| n.as_str()) == Some("Bash"))
        .filter_map(|b| block_input_str(b, &["command", "cmd"]))
        .map(|c| c.chars().take(100).collect())
        .collect()
}

pub(super) fn tool_counts(blocks: &[&Value]) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for block in blocks {
        let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
        *counts.entry(name.to_string()).or_insert(0) += 1;
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    v
}

fn format_duration(total_sec: i64) -> String {
    let hours = total_sec / 3600;
    let minutes = (total_sec % 3600) / 60;
    let seconds = total_sec % 60;
    if hours > 0 {
        return format!("{hours}h {minutes}m");
    }
    if minutes > 0 {
        return format!("{minutes}m {seconds}s");
    }
    format!("{seconds}s")
}

pub(super) fn compute_duration(turns: &[Value]) -> Option<String> {
    let timestamps: Vec<chrono::DateTime<chrono::Utc>> = turns
        .iter()
        .filter_map(|t| t.get("timestamp")?.as_str())
        .filter_map(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .collect();
    if timestamps.len() < 2 {
        return None;
    }
    let diff_sec = (*timestamps.last()? - *timestamps.first()?).num_seconds();
    (diff_sec >= 0).then(|| format_duration(diff_sec))
}
