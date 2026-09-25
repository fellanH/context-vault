//! Per-kind title/body extraction from a parsed markdown entry. Faithful port of
//! the TS `parseEntryFromMarkdown`. Splitting frontmatter parsing (`fm_parse.rs`)
//! from this keeps each file within the structural-contract function ceiling.

use regex::Regex;
use serde_json::{Map, Value};

use crate::fm_parse::extract_custom_meta;

pub struct ParseEntryResult {
    pub title: Option<String>,
    pub body: String,
    pub meta: Option<Map<String, Value>>,
}

fn fm_title(fm_meta: &Map<String, Value>) -> Option<String> {
    fm_meta.get("title").and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn first_heading(body: &str) -> Option<String> {
    Regex::new(r"^#+\s+(.+)")
        .unwrap()
        .captures(body)
        .map(|c| c[1].trim().to_string())
}

pub fn parse_entry_from_markdown(
    kind: &str,
    body: &str,
    fm_meta: &Map<String, Value>,
) -> ParseEntryResult {
    match kind {
        "decision" => parse_decision(body, fm_meta),
        "pattern" => parse_pattern(body, fm_meta),
        // insight and the default fall through to the same heading-based logic.
        _ => ParseEntryResult {
            title: fm_title(fm_meta).or_else(|| first_heading(body)),
            body: body.to_string(),
            meta: extract_custom_meta(fm_meta),
        },
    }
}

fn parse_decision(body: &str, fm_meta: &Map<String, Value>) -> ParseEntryResult {
    let title_re = Regex::new(r"^## Decision\s*\n+([\s\S]*?)(?:\n##|\n*\z)").unwrap();
    let rationale_re = Regex::new(r"## Rationale\s*\n+([\s\S]*)").unwrap();
    let title = title_re
        .captures(body)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_else(|| body.chars().take(100).collect());
    let rationale = rationale_re
        .captures(body)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_else(|| body.to_string());
    ParseEntryResult { title: Some(title), body: rationale, meta: extract_custom_meta(fm_meta) }
}

fn parse_pattern(body: &str, fm_meta: &Map<String, Value>) -> ParseEntryResult {
    let title_re = Regex::new(r"^# (.+)").unwrap();
    let code_re = Regex::new(r"```[\w]*\n([\s\S]*?)```").unwrap();
    let title = title_re
        .captures(body)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_else(|| body.chars().take(80).collect());
    let content = code_re
        .captures(body)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_else(|| body.to_string());
    ParseEntryResult { title: Some(title), body: content, meta: extract_custom_meta(fm_meta) }
}
