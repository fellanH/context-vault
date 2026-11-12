//! Incremental reindex: only files changed since the last run are re-parsed, rows for
//! deleted files are removed, renames are followed, and a vault that looks missing
//! is never pruned. Uses NoOpEmbedder (no model download).

use context_vault_core::backends::{NoOpEmbedder, SqliteBackend};
use context_vault_core::engine::{ReindexOptions, SaveInput, VaultEngine};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

async fn backend(root: &Path) -> (SqliteBackend, PathBuf) {
    let vault_dir = root.join("vault");
    std::fs::create_dir_all(&vault_dir).unwrap();
    let db = root.join("vault.db");
    let e = SqliteBackend::new(db.to_str().unwrap(), vault_dir.to_str().unwrap(), Arc::new(NoOpEmbedder)).unwrap();
    e.init().await.unwrap();
    (e, vault_dir)
}

async fn save(e: &SqliteBackend, title: &str) -> String {
    let input = SaveInput {
        kind: "insight".to_string(),
        title: Some(title.to_string()),
        body: format!("body of {title}"),
        ..Default::default()
    };
    let id = e.save(input).await.unwrap().id;
    e.get(&id).await.unwrap().unwrap().file_path.unwrap()
}

async fn incremental(e: &SqliteBackend) -> context_vault_core::engine::ReindexSummary {
    e.reindex(ReindexOptions { full: false }).await.unwrap()
}

#[tokio::test]
async fn skips_unchanged_follows_edits_renames_and_deletes() {
    let tmp = tempfile::tempdir().unwrap();
    let (e, _vault) = backend(tmp.path()).await;
    let a = save(&e, "Alpha").await;
    let b = save(&e, "Bravo").await;
    let c = save(&e, "Charlie").await;

    // Let the files age past the watermark's slack (2 s) so the next run's watermark
    // is clearly newer than them. First run has no watermark: everything is parsed.
    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert_eq!(incremental(&e).await.entries_reindexed, 3);

    let s = incremental(&e).await;
    assert_eq!((s.entries_reindexed, s.files_seen, s.rows_removed), (0, 3, 0), "nothing changed");

    // Edit one, rename one, delete one.
    std::fs::write(&a, std::fs::read_to_string(&a).unwrap().replace("body of Alpha", "edited alpha")).unwrap();
    let b_moved = b.replace("bravo", "bravo-moved");
    assert_ne!(b_moved, b, "file name contains the slug");
    std::fs::rename(&b, &b_moved).unwrap();
    std::fs::remove_file(&c).unwrap();

    let s = incremental(&e).await;
    assert_eq!(s.entries_reindexed, 2, "edited + renamed re-parsed, untouched skipped");
    assert_eq!(s.rows_removed, 1, "deleted file's row removed");
    assert!(s.prune_refused.is_none());

    let rows = e.list(context_vault_core::engine::ListOptions { limit: 100, ..Default::default() }).await.unwrap();
    let paths: Vec<String> = rows.iter().filter_map(|r| r.file_path.clone()).collect();
    assert_eq!(rows.len(), 2);
    assert!(paths.contains(&b_moved), "renamed row follows the file: {paths:?}");
    assert!(!paths.contains(&c));
    assert!(rows.iter().any(|r| r.body.contains("edited alpha")), "edit picked up");
}

#[tokio::test]
async fn missing_vault_refuses_to_prune() {
    let tmp = tempfile::tempdir().unwrap();
    let (e, vault) = backend(tmp.path()).await;
    for t in ["One", "Two", "Three"] {
        save(&e, t).await;
    }
    incremental(&e).await;

    // Vault dir gone (unmounted, wrong path): the index must survive.
    std::fs::rename(&vault, tmp.path().join("elsewhere")).unwrap();
    let s = incremental(&e).await;
    assert_eq!(s.rows_removed, 0);
    assert!(s.prune_refused.is_some(), "refusal is reported, not silent");
    assert_eq!(e.stats().await.unwrap().total_entries, 3);
}

#[tokio::test]
async fn full_reparses_everything() {
    let tmp = tempfile::tempdir().unwrap();
    let (e, _vault) = backend(tmp.path()).await;
    save(&e, "One").await;
    save(&e, "Two").await;
    incremental(&e).await;
    let s = e.reindex(ReindexOptions { full: true }).await.unwrap();
    assert_eq!(s.entries_reindexed, 2);
}

#[tokio::test]
async fn fresh_rebuild_restores_typed_metadata_and_tracks_metadata_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let (e, vault) = backend(tmp.path()).await;
    let path = vault.join("evidence.md");
    std::fs::write(&path, "---\nid: evidence\nkind: insight\ncreated: 2026-10-12T00:00:00Z\namount: 9\nenabled: true\nsource_ref: pricing-page\nverified_at: 2026-10-12\n---\nEvidence body").unwrap();
    e.reindex(ReindexOptions { full: true }).await.unwrap();
    let entry = e.get("evidence").await.unwrap().unwrap();
    let meta = entry.meta.unwrap();
    assert_eq!(meta["amount"], serde_json::json!(9));
    assert_eq!(meta["enabled"], serde_json::json!(true));
    assert_eq!(meta["source_ref"], serde_json::json!("pricing-page"));
    std::fs::write(&path, "---\nid: evidence\nkind: insight\namount: 12\nsource_ref: corrected-page\n---\nEvidence body").unwrap();
    e.reindex(ReindexOptions { full: true }).await.unwrap();
    let meta = e.get("evidence").await.unwrap().unwrap().meta.unwrap();
    assert_eq!(meta["amount"], serde_json::json!(12));
    assert_eq!(meta["source_ref"], serde_json::json!("corrected-page"));
}
