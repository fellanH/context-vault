//! Write handlers: save_context, delete_context, clear_context, session_end.
//! These modify the vault state.

use std::sync::Arc;
use serde_json::Value;
use crate::engine::{VaultEngine, SaveInput};

/// Implements save_context: save or update an entry.
pub async fn handle_save_context(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let kind = args
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or("Required: kind")?
        .to_string();
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("Required: body")?
        .to_string();
    let title = args.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
    let id = args.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Parse tags array
    let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    });

    let input = SaveInput {
        id,
        kind,
        title,
        body,
        tags,
        meta: args.get("meta").cloned(),
        source: args.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
        identity_key: args.get("identity_key").and_then(|v| v.as_str()).map(|s| s.to_string()),
        tier: args.get("tier").and_then(|v| v.as_str()).map(|s| s.to_string()),
        ..Default::default()
    };

    let output = engine.save(input).await.map_err(|e| e.to_string())?;

    Ok(format!("✓ Saved entry: `{}`", output.id))
}

/// Implements delete_context: remove an entry by ID.
pub async fn handle_delete_context(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Required: id")?;

    // First fetch the entry to report details
    let entry = engine
        .get(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or(format!("Entry not found: {}", id))?;

    engine.delete(id).await.map_err(|e| e.to_string())?;

    let title = entry.title.as_deref().unwrap_or("(untitled)");
    Ok(format!(
        "✓ Deleted {} `{}` [{}]",
        entry.kind, title, entry.id
    ))
}

/// Implements session_end: capture session summary and optional discoveries as vault entries.
/// PARITY: The TS version prompts the user; we accept pre-filled summary + discoveries.
pub async fn handle_session_end(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let summary = args.get("summary").and_then(|v| v.as_str());
    let bucket = args.get("bucket").and_then(|v| v.as_str());

    let mut results = vec![];

    // Save session summary if provided
    if let Some(summary_text) = summary {
        let input = crate::engine::SaveInput {
            kind: "session".to_string(),
            body: summary_text.to_string(),
            title: Some("Session Summary".to_string()),
            tags: bucket.map(|b| vec![format!("bucket:{}", b)]),
            tier: Some("ephemeral".to_string()),
            ..Default::default()
        };

        match engine.save(input).await {
            Ok(output) => results.push(format!("✓ Saved session summary: `{}`", output.id)),
            Err(e) => results.push(format!("⚠ Failed to save summary: {}", e)),
        }
    }

    // Save any discoveries as individual entries
    if let Some(discoveries) = args.get("discoveries").and_then(|v| v.as_array()) {
        for (i, disc) in discoveries.iter().enumerate() {
            let kind = disc
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("insight");
            let title = disc.get("title").and_then(|v| v.as_str());
            let body = disc.get("body").and_then(|v| v.as_str()).ok_or(
                format!("Discovery {} missing body", i)
            )?;

            let input = crate::engine::SaveInput {
                kind: kind.to_string(),
                body: body.to_string(),
                title: title.map(|t| t.to_string()),
                tags: bucket.map(|b| vec![format!("bucket:{}", b)]),
                ..Default::default()
            };

            match engine.save(input).await {
                Ok(output) => results.push(format!("✓ Saved discovery: `{}`", output.id)),
                Err(e) => results.push(format!("⚠ Failed to save discovery {}: {}", i, e)),
            }
        }
    }

    if results.is_empty() {
        return Ok("Session ended (no summaries or discoveries provided)".to_string());
    }

    Ok(results.join("\n"))
}
