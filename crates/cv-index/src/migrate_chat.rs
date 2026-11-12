//! Claude Code transcripts (`events/chat-logs/**/*.jsonl`) → one `chat-message` record per
//! user or assistant message with text. Tool calls, tool results and bookkeeping lines are
//! counted, not converted.
use super::{sink::Sink, verify::Sample, Collected, MigrateOptions, MigrateReport};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use cv_core::Record;
use serde::Serialize;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub(crate) const CHAT_DIR: &str = "events/chat-logs";
pub(crate) const KIND: &str = "chat-message";

#[derive(Default, Serialize)]
pub struct ChatReport {
    pub files: u64,
    pub lines: u64,
    pub messages: u64,
    /// Lines that are not a user/assistant message with text (tool calls, results, metadata).
    pub not_messages: u64,
    /// Lines that are not JSON or not UTF-8.
    pub unparseable: u64,
}

pub(crate) fn convert(options: &MigrateOptions, report: &mut MigrateReport, collected: &mut Collected) -> Result<()> {
    let mut sink = Sink::new(&options.exhaust);
    let root = options.main.join(CHAT_DIR);
    for entry in walkdir::WalkDir::new(&root).sort_by_file_name() {
        let entry = entry?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|e| e == "jsonl") {
            report.chat.files += 1;
            let rel = entry.path().strip_prefix(&options.main)?;
            convert_file(entry.path(), rel, report, collected, &mut sink)?;
        }
    }
    sink.finish()
}

fn convert_file(path: &Path, rel: &Path, report: &mut MigrateReport, collected: &mut Collected, sink: &mut Sink) -> Result<()> {
    let reader = BufReader::new(std::fs::File::open(path)?);
    for (line_no, bytes) in reader.split(b'\n').enumerate() {
        report.chat.lines += 1;
        let parsed = String::from_utf8(bytes?).ok().and_then(|line| message(&line).ok());
        match parsed {
            None => report.chat.unparseable += 1,
            Some(None) => report.chat.not_messages += 1,
            Some(Some(record)) => keep(record, rel, line_no, report, collected, sink)?,
        }
    }
    Ok(())
}

fn keep(record: Record, rel: &Path, line: usize, report: &mut MigrateReport, collected: &mut Collected, sink: &mut Sink) -> Result<()> {
    // A resumed session repeats earlier messages under the same uuid.
    if !collected.ids.insert(record.id.clone()) {
        report.duplicate_ids += 1;
        return Ok(());
    }
    report.chat.messages += 1;
    *report.converted.entry(KIND.into()).or_default() += 1;
    collected.sample(Sample::Chat { rel: rel.to_path_buf(), line, id: record.id.clone() });
    sink.push(KIND.into(), record)
}

/// `Ok(None)`: valid JSON that is not a message with text.
pub(crate) fn message(line: &str) -> Result<Option<Record>> {
    let v: Value = serde_json::from_str(line)?;
    let role = v["type"].as_str().filter(|t| matches!(*t, "user" | "assistant"));
    let (Some(role), Some(id), Some(ts)) = (role, v["uuid"].as_str(), v["timestamp"].as_str()) else {
        return Ok(None);
    };
    let text = text_of(&v["message"]["content"]);
    if text.trim().is_empty() {
        return Ok(None);
    }
    let mut record = Record::new(KIND, &text);
    record.id = id.to_string();
    record.created = DateTime::parse_from_rfc3339(ts).context("timestamp")?.with_timezone(&Utc);
    record.source = Some("claude-code".into());
    record.meta.insert("role".into(), role.into());
    if let Some(session) = v["sessionId"].as_str() {
        record.meta.insert("session".into(), session.into());
    }
    Ok(Some(record))
}

/// A string, or the `text` blocks of a content array joined by blank lines.
fn text_of(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    }
}
