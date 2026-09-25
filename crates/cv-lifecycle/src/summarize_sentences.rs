//! Sentence extraction and condensed-tier selection for `generate_summary_tiers`.

use super::{abbrev_regex, list_number_regex};

pub(super) fn extract_sentences(cleaned: &str) -> Vec<String> {
    let mut sentences: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in cleaned.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("```") {
            if !current.trim().is_empty() {
                sentences.push(current.trim().to_string());
                current.clear();
            }
            continue;
        }

        let content = trimmed
            .strip_prefix(|c| matches!(c, '-' | '*' | '+'))
            .and_then(|s| s.strip_prefix(char::is_whitespace))
            .unwrap_or(trimmed);
        let content = list_number_regex().split(content).next().unwrap_or(content);

        current.push_str(if current.is_empty() { "" } else { " " });
        current.push_str(content);

        current = consume_sentences(&current, &mut sentences);
    }
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }
    sentences
}

fn is_boundary_char(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?')
}

/// Whether `after_idx` sits at whitespace or end-of-string — i.e. a real
/// sentence-ending punctuation mark rather than e.g. a decimal point.
fn followed_by_whitespace_or_end(current: &str, after_idx: usize) -> bool {
    after_idx >= current.len()
        || current[after_idx..].chars().next().is_some_and(char::is_whitespace)
}

/// Push `current[last_pos..after_idx]` onto `sentences` unless it's empty or
/// an abbreviation false-positive. Returns the new `last_pos` on success.
fn push_sentence_if_valid(
    current: &str,
    last_pos: usize,
    after_idx: usize,
    sentences: &mut Vec<String>,
) -> Option<usize> {
    let sent = current[last_pos..after_idx].trim();
    if sent.is_empty() || abbrev_regex().is_match(sent) {
        return None;
    }
    sentences.push(sent.to_string());
    Some(after_idx)
}

/// Split `.`/`!`/`?`-terminated sentences off the front of `current` into
/// `sentences`, returning the unterminated remainder.
fn consume_sentences(current: &str, sentences: &mut Vec<String>) -> String {
    let mut last_pos = 0;
    for (i, ch) in current.chars().enumerate() {
        if !is_boundary_char(ch) {
            continue;
        }
        let after_idx = i + ch.len_utf8();
        if after_idx <= last_pos || !followed_by_whitespace_or_end(current, after_idx) {
            continue;
        }
        if let Some(new_pos) = push_sentence_if_valid(current, last_pos, after_idx, sentences) {
            last_pos = new_pos;
        }
    }
    if last_pos < current.len() {
        current[last_pos..].to_string()
    } else {
        String::new()
    }
}

pub(super) fn pick_condensed(sentences: &[String], cleaned: &str, cap_len: usize) -> String {
    if sentences.is_empty() {
        cleaned[..std::cmp::min(cap_len, cleaned.len())].to_string()
    } else if sentences.len() == 1 {
        sentences[0].clone()
    } else {
        let first = &sentences[0];
        let last = &sentences[sentences.len() - 1];
        if first == last {
            first.clone()
        } else {
            format!("{} {}", first, last)
        }
    }
}
