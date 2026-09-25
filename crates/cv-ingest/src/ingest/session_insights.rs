//! Auto-insight extraction: scans assistant turns for a configurable marker pattern
//! (default `★ Insight`) followed by a fenced block, matching the legacy TS hook's
//! regex-based extraction. Pure -- dedup against already-saved entries is the CLI
//! layer's job (it owns the DB connection).

use regex::Regex;
use serde_json::Value;

use super::session_transcript::extract_text;

#[derive(Debug, Clone)]
pub struct InsightCandidate {
    pub title: String,
    pub body: String,
}

fn is_assistant_turn(turn: &Value) -> bool {
    turn.get("role").and_then(|r| r.as_str()) == Some("assistant")
}

fn build_insight_regex(patterns: &[&str]) -> Option<Regex> {
    let escaped: Vec<String> = patterns.iter().map(|p| regex::escape(p)).collect();
    let pattern = format!(r"(?:{})[─\s]*`?\n([\s\S]*?)\n`?─{{10,}}", escaped.join("|"));
    Regex::new(&pattern).ok()
}

fn insight_title(body: &str) -> String {
    let bold = Regex::new(r"\*\*(.+?)\*\*").ok().and_then(|re| {
        re.captures(body).map(|cap| cap[1].chars().take(80).collect::<String>())
    });
    bold.unwrap_or_else(|| {
        let first_line = body.lines().next().unwrap_or("").replace("**", "");
        first_line.trim().chars().take(80).collect()
    })
}

fn candidates_in_turn(turn: &Value, re: &Regex, min_chars: usize) -> Vec<InsightCandidate> {
    if !is_assistant_turn(turn) {
        return Vec::new();
    }
    let text = extract_text(turn);
    re.captures_iter(&text)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|body| body.chars().count() >= min_chars)
        .map(|body| {
            let title = insight_title(&body);
            InsightCandidate { title, body }
        })
        .collect()
}

/// Extract up to `max_per_session` insight blocks from assistant turns. Matches the
/// legacy default marker (`★ Insight`) unless `patterns` overrides it.
pub fn extract_insights(
    turns: &[Value],
    patterns: &[&str],
    min_chars: usize,
    max_per_session: usize,
) -> Vec<InsightCandidate> {
    let Some(re) = build_insight_regex(patterns) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for turn in turns {
        for candidate in candidates_in_turn(turn, &re, min_chars) {
            if out.len() >= max_per_session {
                return out;
            }
            out.push(candidate);
        }
    }
    out
}

#[cfg(test)]
#[path = "session_insights_tests.rs"]
mod tests;
