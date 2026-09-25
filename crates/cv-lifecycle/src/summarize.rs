//! Two-tier text summarization for vault entries (condensed + keypoint).
//! Focuses on string/regex work; no DB access required.

use once_cell::sync::Lazy;
use regex::Regex;

#[path = "summarize_sentences.rs"]
mod sentences;
use sentences::{extract_sentences, pick_condensed};

const CONDENSED_CAP: usize = 300;
const KEYPOINT_CAP: usize = 150;
const SHORT_THRESHOLD: usize = 150;

/// Regex for common abbreviations that shouldn't trigger sentence splits.
fn abbrev_regex() -> &'static Regex {
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i:mr|mrs|ms|dr|prof|sr|jr|vs|etc|i\.e|e\.g|approx|dept|est|inc|ltd|corp)\.\s*$")
            .unwrap());
    &RE
}

/// Regex for matching list item prefixes (numbered).
fn list_number_regex() -> &'static Regex {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+\.\s+").unwrap());
    &RE
}

/// Regex for matching headers (without lookahead, which is unsupported).
fn header_regex() -> &'static Regex {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^#{1,6}\s+(.+)").unwrap());
    &RE
}

fn strip_frontmatter(text: &str) -> String {
    if !text.starts_with("---") {
        return text.to_string();
    }
    let Some(end) = text.find("\n---") else {
        return text.to_string();
    };
    text[(end + 4)..].trim_start().to_string()
}

const CODE_LINE_PREFIXES: &[&str] = &[
    "```", "//", "/*", "import ", "export ", "const ", "let ", "function ",
    "class ", "{", "}", "def ", "return ",
];

fn is_code_line(line: &str) -> bool {
    let t = line.trim();
    CODE_LINE_PREFIXES.iter().any(|p| t.starts_with(p))
}

fn extract_comment_from_line(trimmed: &str) -> Option<String> {
    if let Some(rest) = trimmed.strip_prefix("//") {
        return Some(rest.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix('#') {
        if !trimmed.starts_with("##") {
            return Some(rest.trim().to_string());
        }
    }
    if trimmed.starts_with("/*") {
        let content = trimmed.strip_prefix("/*").unwrap_or("").strip_suffix("*/").unwrap_or("").trim();
        if !content.is_empty() {
            return Some(content.to_string());
        }
    }
    None
}

fn cap(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let truncated = &text[..std::cmp::min(limit.saturating_sub(3), text.len())];
    if let Some(pos) = truncated.rfind(' ') {
        if pos > limit / 2 {
            return format!("{}...", &truncated[..pos]);
        }
    }
    format!("{}...", truncated)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SummaryTiers {
    pub condensed: String,
    pub keypoint: String,
}

/// Condensed+keypoint summary for entries where most lines look like code —
/// uses the first code comment, falling back to a labeled snippet of the
/// first non-blank line. Returns `None` when the body isn't code-dominated.
fn code_only_summary(cleaned: &str) -> Option<SummaryTiers> {
    let lines: Vec<&str> = cleaned.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return None;
    }
    let code_ratio = lines.iter().filter(|l| is_code_line(l)).count() as f32 / lines.len() as f32;
    if code_ratio <= 0.7 {
        return None;
    }

    let comment = cleaned.lines().find_map(|line| extract_comment_from_line(line.trim()));
    let first_line = cleaned.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    let label = comment.unwrap_or_else(|| {
        format!("Code block: {}", &first_line[..std::cmp::min(80, first_line.len())])
    });
    Some(SummaryTiers {
        condensed: cap(&label, CONDENSED_CAP),
        keypoint: cap(&label, KEYPOINT_CAP),
    })
}

/// Generate two-tier summaries (condensed + keypoint) from entry body.
///
/// - Condensed: first sentence + last sentence (capped at 300 chars)
/// - Keypoint: first header or first sentence (capped at 150 chars)
///
/// For short entries (< 150 chars), both tiers use the whole body.
/// For code-only entries, uses the first code comment or code snippet label.
pub fn generate_summary_tiers(body: &str) -> SummaryTiers {
    let cleaned = strip_frontmatter(body).trim().to_string();

    if cleaned.is_empty() {
        return SummaryTiers {
            condensed: String::new(),
            keypoint: String::new(),
        };
    }

    // Short entries: use body as both
    if cleaned.len() < SHORT_THRESHOLD {
        return SummaryTiers {
            condensed: cleaned.clone(),
            keypoint: cleaned,
        };
    }

    if let Some(tiers) = code_only_summary(&cleaned) {
        return tiers;
    }

    let header = cleaned.lines().find_map(|line| {
        header_regex().captures(line).and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
    });

    let sentences = extract_sentences(&cleaned);

    let keypoint = header
        .or_else(|| sentences.first().cloned())
        .unwrap_or_else(|| cleaned[..std::cmp::min(KEYPOINT_CAP, cleaned.len())].to_string());

    let condensed = pick_condensed(&sentences, &cleaned, CONDENSED_CAP);

    SummaryTiers {
        condensed: cap(&condensed, CONDENSED_CAP),
        keypoint: cap(&keypoint, KEYPOINT_CAP),
    }
}

#[cfg(test)]
#[path = "summarize_tests.rs"]
mod tests;
