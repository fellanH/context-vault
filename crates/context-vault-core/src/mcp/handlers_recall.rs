//! The `recall` handler: lightweight passive hints from a raw signal (prompt,
//! error, file path, task). Split from `handlers_read.rs`; recall carries its own
//! selection policy (project scoping, score floor, dedup) on top of hybrid search.

use std::sync::Arc;
use serde_json::Value;
use crate::engine::VaultEngine;

/// Implements recall: lightweight hint surface from signal (prompt, error, file path, task).
/// PARITY: Full session dedup map and co-retrieval weighting require state management.
pub async fn handle_recall(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let signal = args
        .get("signal")
        .and_then(|v| v.as_str())
        .ok_or("Required: signal")?;
    let max_hints = args
        .get("max_hints")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;
    let bucket = args
        .get("bucket")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    // Extract keywords from signal
    let keywords = extract_keywords(signal);
    if keywords.is_empty() {
        return Ok("No relevant entries found.".to_string());
    }

    // Search for entries matching keywords. Over-fetch past max_hints so the
    // score floor and title dedup below don't starve the hint count.
    let query = keywords.join(" ");
    let opts = crate::engine::SearchOptions {
        scope_project: bucket.map(|b| b.to_string()),
        limit: (max_hints * 5).max(15),
        offset: 0,
        ..Default::default()
    };

    let results = engine
        .hybrid_search(&query, opts)
        .await
        .map_err(|e| e.to_string())?;

    let hints = select_hints(results, max_hints);
    if hints.is_empty() {
        return Ok(format!("No hints found for: {}", signal.chars().take(50).collect::<String>()));
    }

    let mut lines = vec![format!("## Hints ({} relevant)\n", hints.len())];
    for r in hints {
        let e = &r.entry;
        let title = e.title.as_deref().unwrap_or("(untitled)");
        lines.push(format!("- **{}** [`{}`]", title, e.kind));
    }

    Ok(lines.join("\n"))
}

/// Hints are passive context injection, so weak matches cost more than missing
/// ones. RRF scores are rank-based (rank-1 single-lane ≈ 1.67 × tier/heat/recency
/// boosts), so this floor drops stale, low-ranked tail results — it cannot judge
/// whether a rank-1 match is semantically relevant; project scoping handles the
/// cross-project case.
const MIN_HINT_SCORE: f32 = 0.4;

/// Post-select fused search results into hint-worthy entries: apply the score
/// floor, drop untitled entries (a hint rendered as "(untitled)" carries no
/// information), dedup near-identical entries by (title, kind) — duplicate saves
/// are common and two hints with the same title carry no extra signal — and
/// truncate to `max_hints`. Results arrive sorted by score descending, so the
/// kept duplicate is always the strongest one.
fn select_hints(
    results: Vec<crate::engine::ScoredEntry>,
    max_hints: usize,
) -> Vec<crate::engine::ScoredEntry> {
    let mut seen = std::collections::HashSet::new();
    results
        .into_iter()
        .filter(|r| r.score >= MIN_HINT_SCORE)
        .filter(|r| r.entry.title.as_deref().is_some_and(|t| !t.trim().is_empty()))
        .filter(|r| seen.insert((r.entry.title.clone(), r.entry.kind.clone())))
        .take(max_hints)
        .collect()
}

/// Extract keywords from a signal string (remove stop words, etc.).
fn extract_keywords(signal: &str) -> Vec<String> {
    let stopwords = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "need", "must", "and", "or",
        "not", "for", "with", "from", "to", "of", "in", "on", "at", "by",
    ];

    let words: Vec<_> = signal
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|w| w.len() >= 2 && !stopwords.contains(w))
        .take(10)
        .map(|w| w.to_string())
        .collect();

    // Deduplicate
    let mut seen = std::collections::HashSet::new();
    words
        .into_iter()
        .filter(|w| seen.insert(w.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Entry, ScoredEntry};

    fn scored(title: &str, kind: &str, score: f32) -> ScoredEntry {
        ScoredEntry {
            entry: Entry {
                id: format!("{title}-{score}"),
                kind: kind.to_string(),
                category: "events".to_string(),
                title: (!title.is_empty()).then(|| title.to_string()),
                body: String::new(),
                tags: vec![],
                meta: None,
                source: None,
                identity_key: None,
                tier: "working".to_string(),
                indexed: true,
                created_at: String::new(),
                updated_at: String::new(),
                expires_at: None,
                superseded_by: None,
                supersedes: None,
                related_to: None,
                hit_count: 0,
                recall_count: 0,
                recall_sessions: 0,
                last_recalled_at: None,
                heat_tier: None,
                summary_condensed: None,
                summary_keypoint: None,
                file_path: None,
                last_accessed_at: None,
                source_files: None,
            },
            score,
            fts_rank: None,
            vec_rank: None,
        }
    }

    #[test]
    fn select_hints_drops_below_floor_dedups_and_truncates() {
        let results = vec![
            scored("charter", "event", 1.6),
            scored("charter", "event", 1.5), // duplicate title+kind — dropped
            scored("charter", "insight", 1.4), // same title, different kind — kept
            scored("", "browser-screenshot", 1.3), // untitled — dropped
            scored("weak tail", "event", MIN_HINT_SCORE - 0.01), // below floor — dropped
            scored("third", "event", 0.9),
            scored("fourth", "event", 0.8), // beyond max_hints — truncated
        ];
        let hints = select_hints(results, 3);
        let ids: Vec<_> = hints.iter().map(|h| (h.entry.title.clone().unwrap(), h.entry.kind.clone())).collect();
        assert_eq!(
            ids,
            vec![
                ("charter".to_string(), "event".to_string()),
                ("charter".to_string(), "insight".to_string()),
                ("third".to_string(), "event".to_string()),
            ]
        );
    }
}
