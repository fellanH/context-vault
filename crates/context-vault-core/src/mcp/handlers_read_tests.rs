use super::handlers_read::{handle_get_context, handle_list_context};
use super::handlers_write::handle_save_context;
use crate::backends::{NoOpEmbedder, SqliteBackend};
use crate::engine::VaultEngine;
use serde_json::json;
use std::sync::Arc;

async fn engine(tmp: &tempfile::TempDir) -> Arc<dyn VaultEngine> {
    let vault_dir = tmp.path().join("vault");
    std::fs::create_dir_all(&vault_dir).unwrap();
    let e = SqliteBackend::new(tmp.path().join("vault.db").to_str().unwrap(), vault_dir.to_str().unwrap(), Arc::new(NoOpEmbedder)).unwrap();
    e.init().await.unwrap();
    Arc::new(e)
}

/// Card 18: get_context's advertised filters and output budget take effect.
#[tokio::test]
async fn get_context_applies_filters_and_budget() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = engine(&tmp).await;
    for (kind, tag, body) in [
        ("decision", "infra", "widget decision about the rollout plan"),
        ("insight", "infra", "widget insight about caching layers"),
        ("insight", "ui", "widget insight about the button colours"),
    ] {
        handle_save_context(engine.clone(), &json!({"kind": kind, "title": "Widget", "body": body, "tags": [tag]})).await.unwrap();
    }

    let kind = handle_get_context(engine.clone(), &json!({"query": "widget", "kind": "insight"})).await.unwrap();
    assert!(kind.starts_with("## Results (2 matches)"), "{kind}");
    assert!(!kind.contains("rollout"));

    let tagged = handle_get_context(engine.clone(), &json!({"query": "widget", "kind": "insight", "tags": ["ui"]})).await.unwrap();
    assert!(tagged.starts_with("## Results (1 matches)") && tagged.contains("button"), "{tagged}");

    let cut = handle_get_context(engine.clone(), &json!({"query": "widget", "kind": "insight", "tags": ["ui"], "body_limit": 6})).await.unwrap();
    assert!(cut.contains("\n  widget\n") || cut.ends_with("\n  widget"), "{cut}");

    let budget = handle_get_context(engine.clone(), &json!({"query": "widget", "max_tokens": 1})).await.unwrap();
    assert_eq!(budget.matches("- **").count(), 1, "always shows at least one result: {budget}");
    assert!(budget.contains("2 more not shown: max_tokens reached"), "{budget}");

    let listed = handle_list_context(engine.clone(), &json!({"category": "nope"})).await.unwrap();
    assert!(listed.starts_with("No entries found"), "{listed}");
}

/// Card 30 (via 18): save_context's supersedes/expires_at/related_to are stored,
/// and supersedes marks the OLD entry, so search returns the replacement.
#[tokio::test]
async fn save_supersedes_hides_the_old_entry_not_the_new_one() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = engine(&tmp).await;
    let id = |reply: String| reply.split('`').nth(1).unwrap().to_string();
    let old = id(handle_save_context(engine.clone(), &json!({"kind": "decision", "title": "Gadget plan", "body": "gadget ships in May"})).await.unwrap());
    let new = id(handle_save_context(engine.clone(), &json!({
        "kind": "decision", "title": "Gadget plan v2", "body": "gadget ships in June",
        "supersedes": [old], "related_to": [old], "expires_at": "2999-01-01T00:00:00Z"
    })).await.unwrap());

    let found = handle_get_context(engine.clone(), &json!({"query": "gadget"})).await.unwrap();
    assert!(found.contains(&new) && !found.contains(&old), "{found}");
    let all = handle_get_context(engine.clone(), &json!({"query": "gadget", "include_superseded": true})).await.unwrap();
    assert!(all.contains(&new) && all.contains(&old), "{all}");

    let stored = engine.get(&new).await.unwrap().unwrap();
    assert_eq!(stored.expires_at.as_deref(), Some("2999-01-01T00:00:00Z"));
    assert_eq!(stored.related_to, Some(vec![old.clone()]));
    assert_eq!(stored.superseded_by, None);
    assert_eq!(engine.get(&old).await.unwrap().unwrap().superseded_by, Some(new));
}
