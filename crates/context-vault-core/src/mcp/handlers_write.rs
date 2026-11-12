//! Write handlers: save_context, delete_context, clear_context, session_end.
//! These modify the vault state.

use std::sync::Arc;
use serde_json::Value;
use crate::engine::{VaultEngine, SaveInput};
use crate::engine::situation_meta::apply_to_meta;

/// Implements save_context: save a new entry, or update one in place when `id` is set.
/// An update keeps the entry's id, kind, file and created time; fields not passed keep their stored value.
pub async fn handle_save_context(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let str_arg = |k: &str| args.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    let str_list = |k: &str| {
        args.get(k).and_then(|v| v.as_array()).map(|arr| {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>()
        })
    };
    let tags = str_list("tags");

    if let Some(id) = str_arg("id") {
        let existing = engine
            .get(&id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or(format!("Entry not found: {}", id))?;
        if let Some(kind) = str_arg("kind") {
            if kind != existing.kind {
                return Err(format!("Cannot change kind on update ({} -> {}); save a new entry instead", existing.kind, kind));
            }
        }
        let input = SaveInput {
            id: Some(id),
            kind: existing.kind,
            title: str_arg("title").or(existing.title),
            body: str_arg("body").unwrap_or(existing.body),
            tags: tags.or(Some(existing.tags)),
            meta: apply_to_meta(
                args.get("meta").cloned(),
                existing.meta.as_ref(),
                args.get("situations"),
                args.get("situation_order"),
            )?,
            source: str_arg("source").or(existing.source),
            identity_key: str_arg("identity_key").or(existing.identity_key),
            tier: str_arg("tier").or(Some(existing.tier)),
            expires_at: str_arg("expires_at").or(existing.expires_at),
            supersedes: str_list("supersedes"),
            related_to: str_list("related_to").or(existing.related_to),
        };
        let output = engine.save(input).await.map_err(|e| e.to_string())?;
        return Ok(format!("✓ Updated entry: `{}`", output.id));
    }

    let input = SaveInput {
        id: None,
        kind: str_arg("kind").ok_or("Required: kind")?,
        title: str_arg("title"),
        body: str_arg("body").ok_or("Required: body")?,
        tags,
        meta: apply_to_meta(args.get("meta").cloned(), None, args.get("situations"), args.get("situation_order"))?,
        source: str_arg("source"),
        identity_key: str_arg("identity_key"),
        tier: str_arg("tier"),
        expires_at: str_arg("expires_at"),
        supersedes: str_list("supersedes"),
        related_to: str_list("related_to"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{NoOpEmbedder, SqliteBackend};
    use serde_json::json;

    async fn engine(tmp: &tempfile::TempDir) -> Arc<dyn VaultEngine> {
        let vault_dir = tmp.path().join("vault");
        std::fs::create_dir_all(&vault_dir).unwrap();
        let e = SqliteBackend::new(tmp.path().join("vault.db").to_str().unwrap(), vault_dir.to_str().unwrap(), Arc::new(NoOpEmbedder)).unwrap();
        e.init().await.unwrap();
        Arc::new(e)
    }

    fn id_of(reply: &str) -> String {
        reply.split('`').nth(1).unwrap().to_string()
    }

    #[tokio::test]
    async fn save_with_id_updates_in_place() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = engine(&tmp).await;
        let created = handle_save_context(engine.clone(), &json!({"kind": "insight", "title": "WAL mode", "body": "first", "tier": "durable", "tags": ["rust"]})).await.unwrap();
        let id = id_of(&created);
        let before = engine.get(&id).await.unwrap().unwrap();

        // id + body only: no kind needed, same id back, nothing new created.
        let updated = handle_save_context(engine.clone(), &json!({"id": id, "body": "second"})).await.unwrap();
        assert_eq!(id_of(&updated), id);
        let after = engine.get(&id).await.unwrap().unwrap();
        assert_eq!(after.body, "second");
        assert_eq!(after.title.as_deref(), Some("WAL mode"));
        assert_eq!(after.tier, "durable");
        assert_eq!(after.tags, vec!["rust".to_string()]);
        assert_eq!(after.created_at, before.created_at);
        assert_eq!(after.file_path, before.file_path);
        let md = std::fs::read_to_string(after.file_path.unwrap()).unwrap();
        assert!(md.contains("second") && !md.contains("first"));
        let all = engine.list(crate::engine::ListOptions { limit: 10, ..Default::default() }).await.unwrap();
        assert_eq!(all.len(), 1, "update must not create a second entry");

        // Same kind is accepted; a different kind or unknown id is refused.
        assert!(handle_save_context(engine.clone(), &json!({"id": id, "kind": "insight", "body": "third"})).await.is_ok());
        assert!(handle_save_context(engine.clone(), &json!({"id": id, "kind": "decision", "body": "x"})).await.is_err());
        assert!(handle_save_context(engine.clone(), &json!({"id": "01NOPE", "body": "x"})).await.unwrap_err().contains("not found"));
    }
}
