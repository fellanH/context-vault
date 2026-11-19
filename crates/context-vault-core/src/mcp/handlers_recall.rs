//! The `recall` handler (which hits become hints is `recall_select.rs`): lightweight passive hints from a raw signal (prompt,
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
    let area = super::recall_select::area_words(args.get("cwd").and_then(|v| v.as_str()).unwrap_or(""));

    // Extract keywords from signal
    let keywords = extract_keywords(signal);
    if keywords.is_empty() {
        return Ok("No relevant entries found.".to_string());
    }

    // Search for entries matching keywords. Over-fetch well past max_hints: the kinds
    // left out of hints, the score floor and the title dedup thin the list, and the
    // title rerank below needs candidates past the first few (OPS-402: on the 30-prompt
    // bench, useful entries sat at search positions 3 to 39 as often as in the top 3).
    // Any-word hits fill the list, so a long prompt is not left to entries holding
    // every one of its words.
    let query = keywords.join(" ");
    let opts = crate::engine::SearchOptions {
        scope_project: bucket.map(|b| b.to_string()),
        limit: (max_hints * 5).max(40),
        offset: 0,
        fill_any_word: true,
        ..Default::default()
    };

    let results = engine
        .hybrid_search(&query, opts)
        .await
        .map_err(|e| e.to_string())?;

    let hints = super::recall_select::select_hints(results, &super::recall_select::title_words(signal), &area, max_hints);
    if hints.is_empty() {
        return Ok(format!("No hints found for: {}", signal.chars().take(50).collect::<String>()));
    }

    if let Some(session) = args.get("session_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        let ids: Vec<String> = hints.iter().map(|h| h.entry.id.clone()).collect();
        if let Err(e) = engine.log_hints(&ids, session).await {
            eprintln!("[context-vault] hint log: {e:#}");
        }
    }

    let mut lines = vec![format!("## Hints ({} relevant)\n", hints.len())];
    for r in hints {
        let e = &r.entry;
        let title = e.title.as_deref().unwrap_or("(untitled)");
        lines.push(format!("- **{}** [`{}`]", title, e.kind));
    }

    Ok(lines.join("\n"))
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
