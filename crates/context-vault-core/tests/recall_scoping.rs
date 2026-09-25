//! Proves recall's project scoping and near-duplicate dedup. Repro case from a
//! 2026-07-15 dogfood report: one project's recall-hint hook surfaced ANOTHER
//! project's "adopt this charter for our session" prompt-history events on
//! throwaway prompts ("yes go on the mockup"), because recall had no project/cwd
//! scoping and no dedup — in-context it read like a prompt-injection attempt.
//! Uses NoOpEmbedder (FTS lane only), same as search_filters.rs.

use context_vault_core::backends::{NoOpEmbedder, SqliteBackend};
use context_vault_core::engine::{SaveInput, VaultEngine};
use std::sync::Arc;

async fn make() -> (Arc<dyn VaultEngine>, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("vault.db");
    let vault = tmp.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    let e = SqliteBackend::new(db.to_str().unwrap(), vault.to_str().unwrap(), Arc::new(NoOpEmbedder)).unwrap();
    e.init().await.unwrap();
    (Arc::new(e), tmp)
}

fn entry(title: &str, body: &str, tags: Option<Vec<&str>>) -> SaveInput {
    SaveInput {
        kind: "event".to_string(),
        title: Some(title.to_string()),
        body: body.to_string(),
        tags: tags.map(|ts| ts.into_iter().map(String::from).collect()),
        ..Default::default()
    }
}

const CHARTER_TITLE: &str =
    "[other-project] Please adopt this charter for our session:  # other-project";
// Overlaps the repro prompt "yes go on the mockup" on generic short words only.
const CHARTER_BODY: &str =
    "yes go adopt this charter, the mockup of the other project loop compounds";

async fn save_foreign_charter(e: &Arc<dyn VaultEngine>) {
    e.save(entry(
        CHARTER_TITLE,
        CHARTER_BODY,
        Some(vec!["prompt-history", "seat:other-project", "source:other-project"]),
    ))
    .await
    .unwrap();
}

/// The dogfood repro: a foreign project's charter event must not surface as a hint
/// in a session scoped to a different project.
#[tokio::test]
async fn scoped_recall_excludes_foreign_project_entries() {
    let (e, _tmp) = make().await;
    save_foreign_charter(&e).await;

    // Sanity: without scoping the entry is findable (this was the old behavior).
    let unscoped = context_vault_core::mcp::recall(e.clone(), "yes go on the mockup", None, 3)
        .await
        .unwrap();
    assert!(unscoped.contains("other-project"), "entry should be findable unscoped: {unscoped}");

    // Scoped to the session's actual project, the foreign entry must not appear.
    let scoped = context_vault_core::mcp::recall(e.clone(), "yes go on the mockup", Some("glance"), 3)
        .await
        .unwrap();
    assert!(
        !scoped.contains("other-project"),
        "foreign-project charter leaked into scoped recall: {scoped}"
    );
    assert!(scoped.starts_with("No hints found for:"), "expected empty hints: {scoped}");
}

/// Scoping keeps same-project entries (bucket: or seat: match) and global entries
/// (no project-identity tags at all) reachable.
#[tokio::test]
async fn scoped_recall_keeps_own_project_and_global_entries() {
    let (e, _tmp) = make().await;
    save_foreign_charter(&e).await;
    e.save(entry(
        "glance mockup decision",
        "yes go on the mockup for the glance viewer",
        Some(vec!["bucket:glance"]),
    ))
    .await
    .unwrap();
    e.save(entry(
        "global mockup convention",
        "yes go mockup conventions apply everywhere",
        None,
    ))
    .await
    .unwrap();

    let scoped = context_vault_core::mcp::recall(e.clone(), "yes go on the mockup", Some("glance"), 5)
        .await
        .unwrap();
    assert!(scoped.contains("glance mockup decision"), "own-bucket entry missing: {scoped}");
    assert!(scoped.contains("global mockup convention"), "untagged global entry missing: {scoped}");
    assert!(!scoped.contains("other-project"), "foreign entry leaked: {scoped}");
}

/// Near-duplicate saves (same title + kind) collapse to one hint line instead of
/// filling the hint budget with copies.
#[tokio::test]
async fn recall_dedups_hints_by_title_and_kind() {
    let (e, _tmp) = make().await;
    save_foreign_charter(&e).await;
    save_foreign_charter(&e).await;
    save_foreign_charter(&e).await;

    let hints = context_vault_core::mcp::recall(e.clone(), "yes go on the mockup", None, 3)
        .await
        .unwrap();
    let dup_lines = hints.lines().filter(|l| l.contains("other-project")).count();
    assert_eq!(dup_lines, 1, "duplicate-title entries must dedup to one hint: {hints}");
    assert!(hints.contains("## Hints (1 relevant)"), "hint count must reflect dedup: {hints}");
}
