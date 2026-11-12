//! Read-only handlers: get_context, list_context, context_status, list_buckets.
//! These handlers only query the engine; they have no side effects.

use std::sync::Arc;
use serde_json::Value;
use crate::engine::VaultEngine;

/// Implements get_context: search vault for entries by query or filters.
/// Equivalent to TypeScript's get-context.ts but without team/remote merging and
/// advanced features like conflict detection (PARITY: those need infra).
pub async fn handle_get_context(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let query = args.get("query").and_then(|v| v.as_str());
    let str_arg = |k: &str| args.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    let uint_arg = |k: &str| args.get(k).and_then(|v| v.as_u64()).map(|n| n as usize);
    let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>()
    });

    let opts = crate::engine::SearchOptions {
        kind: str_arg("kind"),
        category: str_arg("category"),
        tags: tags.filter(|t| !t.is_empty()),
        since: str_arg("since"),
        until: str_arg("until"),
        include_superseded: args.get("include_superseded").and_then(|v| v.as_bool()).unwrap_or(false),
        limit: uint_arg("limit").unwrap_or(10),
        offset: 0,
        ..Default::default()
    };
    let body_limit = uint_arg("body_limit");
    // Rough budget: ~4 characters per token.
    let max_chars = uint_arg("max_tokens").map(|t| t.saturating_mul(4));

    let results = engine
        .hybrid_search(query.unwrap_or(""), opts)
        .await
        .map_err(|e| e.to_string())?;

    if results.is_empty() {
        return Ok(format!("No results found{}",
            query.map(|q| format!(" for \"{q}\"")).unwrap_or_default()));
    }

    let total = results.len();
    let mut out = format!("## Results ({} matches)\n", total);
    for (shown, r) in results.into_iter().enumerate() {
        let e = &r.entry;
        let title = e.title.as_deref().unwrap_or("(untitled)");
        let mut item = format!(
            "\n- **{}** (score: {:.2}) [{}/{}] id: `{}`",
            title, r.score, e.kind, e.tier, e.id
        );
        if let Some(body) = e.body.lines().next() {
            let body: String = match body_limit {
                Some(n) => body.chars().take(n).collect(),
                None => body.to_string(),
            };
            item.push_str(&format!("\n  {}", body));
        }
        if max_chars.is_some_and(|max| shown > 0 && out.len() + item.len() > max) {
            out.push_str(&format!("\n\n({} more not shown: max_tokens reached)", total - shown));
            break;
        }
        out.push_str(&item);
    }

    Ok(out)
}

/// Implements list_context: list entries without search scoring.
/// Simpler than get_context — just schema-based filtering.
pub async fn handle_list_context(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;
    let kind = args.get("kind").and_then(|v| v.as_str()).map(|s| s.to_string());
    let category = args.get("category").and_then(|v| v.as_str()).map(|s| s.to_string());

    let opts = crate::engine::ListOptions {
        kind,
        category,
        limit,
        offset: 0,
        ..Default::default()
    };

    let entries = engine
        .list(opts)
        .await
        .map_err(|e| e.to_string())?;

    if entries.is_empty() {
        return Ok("No entries found matching filters.".to_string());
    }

    let mut lines = vec![format!("## Entries ({} total)\n", entries.len())];
    for e in entries {
        let title = e.title.as_deref().unwrap_or("(untitled)");
        lines.push(format!(
            "- **{}** [{}/{}] id: `{}`",
            title, e.kind, e.tier, e.id
        ));
    }

    Ok(lines.join("\n"))
}

/// Implements context_status: vault health and stats.
pub async fn handle_context_status(
    engine: Arc<dyn VaultEngine>,
) -> Result<String, String> {
    let stats = engine.stats().await.map_err(|e| e.to_string())?;
    let health = engine.health().await.map_err(|e| e.to_string())?;

    let health_icon = if health.ok { "✓" } else { "⚠" };
    let mut lines = vec![
        format!("## {} Vault Status", health_icon),
        "".to_string(),
        "| | |".to_string(),
        "|---|---|".to_string(),
        format!("| **Total entries** | {} |", stats.total_entries),
        format!("| **Indexed** | {} |", stats.indexed_entries),
        format!("| **Status** | {} |", health.message),
    ];

    if !stats.by_kind.is_empty() {
        lines.push("| **By kind** | |".to_string());
        for (kind, count) in &stats.by_kind {
            lines.push(format!("| · {} | {} |", kind, count));
        }
    }

    Ok(lines.join("\n"))
}

/// Implements list_buckets: list all bucket entities in vault.
pub async fn handle_list_buckets(
    engine: Arc<dyn VaultEngine>,
) -> Result<String, String> {
    let opts = crate::engine::ListOptions {
        kind: Some("bucket".to_string()),
        limit: 1000,
        offset: 0,
        ..Default::default()
    };

    let buckets = engine
        .list(opts)
        .await
        .map_err(|e| e.to_string())?;

    if buckets.is_empty() {
        return Ok("No buckets registered.\n\nCreate one with: save_context(kind: \"bucket\", identity_key: \"bucket:myproject\", title: \"My Project\", body: \"...\")".to_string());
    }

    let mut lines = vec![format!("## Registered Buckets ({})\n", buckets.len())];
    for b in buckets {
        let title = b.title.as_deref().unwrap_or(&b.id);
        let identity = b
            .identity_key
            .as_deref()
            .unwrap_or("(no identity key)");
        lines.push(format!("- **{}** `{}`", title, identity));
        if let Some(body) = &b.body.lines().next() {
            lines.push(format!("  {}", body));
        }
    }

    Ok(lines.join("\n"))
}
