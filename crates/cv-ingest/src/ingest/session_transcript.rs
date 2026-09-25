//! Parses a Claude Code SessionEnd hook payload + its transcript (JSONL) into the
//! signals the legacy TS `session-end` handler captured: exchange count, duration,
//! first/last prompt, files touched, commands run, tool-use counts. Pure -- no I/O, no
//! DB -- so it's fully unit-testable; the CLI layer reads stdin/the transcript file
//! and does the actual vault save.

use serde_json::Value;

use super::session_signal_extract::{
    commands_run, compute_duration, files_modified, is_role, tool_counts, tool_use_blocks,
};

#[derive(Debug, Clone)]
pub struct HookInput {
    pub session_id: Option<String>,
    pub transcript_path: String,
    pub cwd: String,
}

#[derive(Debug, Clone)]
pub struct SessionSignals {
    pub project: String,
    pub message_count: usize,
    pub duration: Option<String>,
    pub first_prompt: String,
    pub last_prompt: String,
    pub files_modified: Vec<String>,
    pub commands_run: Vec<String>,
    pub tool_counts: Vec<(String, usize)>,
}

/// The hook payload Claude Code pipes on stdin: `{session_id, transcript_path, cwd}`.
pub fn parse_hook_input(raw: &str) -> Option<HookInput> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let transcript_path = v.get("transcript_path")?.as_str()?.to_string();
    let cwd = v.get("cwd")?.as_str()?.to_string();
    let session_id = v
        .get("session_id")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    Some(HookInput { session_id, transcript_path, cwd })
}

pub fn parse_turns(transcript_raw: &str) -> Vec<Value> {
    transcript_raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line.trim()).ok())
        .collect()
}

/// Real Claude Code transcripts nest `role`/`content` under a `message` object and
/// interleave non-message line types (`mode`, `attachment`, `system`, `ai-title`, …).
/// Normalize each raw line to the flat `{role, content, timestamp}` shape the extractors
/// expect, dropping every line that isn't a `user`/`assistant` message. `timestamp` is
/// already top-level in real transcripts, so it is carried over verbatim
/// (`compute_duration` keeps reading it unchanged).
pub fn normalize_turns(raw: Vec<Value>) -> Vec<Value> {
    raw.into_iter()
        .filter_map(|line| {
            match line.get("type").and_then(|t| t.as_str()) {
                Some("user") | Some("assistant") => {}
                _ => return None,
            }
            let message = line.get("message")?;
            let mut obj = serde_json::Map::new();
            obj.insert("role".to_string(), message.get("role")?.clone());
            obj.insert("content".to_string(), message.get("content")?.clone());
            if let Some(ts) = line.get("timestamp") {
                obj.insert("timestamp".to_string(), ts.clone());
            }
            Some(Value::Object(obj))
        })
        .collect()
}

pub fn extract_text(turn: &Value) -> String {
    match turn.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// `None` if the transcript has no user turns at all (nothing worth capturing).
pub fn build_signals(turns: &[Value], cwd: &str) -> Option<SessionSignals> {
    let user_turns: Vec<&Value> = turns.iter().filter(|t| is_role(t, "user")).collect();
    if user_turns.is_empty() {
        return None;
    }
    let blocks = tool_use_blocks(turns);
    let message_count = user_turns.len();
    let project = cwd.rsplit('/').next().unwrap_or("unknown").to_string();
    let first_prompt = truncate(&extract_text(user_turns[0]), 200);
    let last_prompt = if message_count > 1 {
        truncate(&extract_text(user_turns[message_count - 1]), 200)
    } else {
        first_prompt.clone()
    };

    Some(SessionSignals {
        project,
        message_count,
        duration: compute_duration(turns),
        first_prompt,
        last_prompt,
        files_modified: files_modified(&blocks),
        commands_run: commands_run(&blocks),
        tool_counts: tool_counts(&blocks),
    })
}

fn push_capped_section(lines: &mut Vec<String>, header: &str, items: &[String], cap: usize) {
    if items.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(header.to_string());
    for item in items.iter().take(cap) {
        lines.push(format!("- {item}"));
    }
    if items.len() > cap {
        lines.push(format!("- ... and {} more", items.len() - cap));
    }
}

pub fn format_session_body(s: &SessionSignals) -> String {
    let duration_part = s.duration.as_ref().map(|d| format!(", ~{d}")).unwrap_or_default();
    let plural = if s.message_count == 1 { "" } else { "s" };

    let mut lines = vec![
        format!("Session in {} ({} exchange{plural}{duration_part}).", s.project, s.message_count),
        String::new(),
        "## What was done".to_string(),
        format!("Opened with: {}", s.first_prompt),
        String::new(),
        format!("Closed with: {}", s.last_prompt),
    ];

    push_capped_section(&mut lines, "## Files modified", &s.files_modified, 20);
    push_capped_section(&mut lines, "## Key commands", &s.commands_run, 10);

    if !s.tool_counts.is_empty() {
        let summary = s
            .tool_counts
            .iter()
            .map(|(name, count)| format!("{name}: {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(String::new());
        lines.push("## Tools used".to_string());
        lines.push(summary);
    }

    lines.join("\n")
}

#[cfg(test)]
#[path = "session_transcript_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "session_transcript_normalize_tests.rs"]
mod normalize_tests;
