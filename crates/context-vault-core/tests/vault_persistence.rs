//! End-to-end proof of Phase 1 local-first markdown persistence: `save` writes a
//! `.md` file to the vault tree and records its `file_path` in the DB, and `reindex`
//! rebuilds the SQLite index by walking that markdown tree. Uses NoOpEmbedder (no
//! model download) — the vec lane is intentionally out of scope here.

use context_vault_core::backends::{NoOpEmbedder, SqliteBackend};
use context_vault_core::engine::{ReindexOptions, SaveInput, VaultEngine};
use std::sync::Arc;

fn insight(body: &str, title: &str) -> SaveInput {
    SaveInput {
        kind: "insight".to_string(),
        title: Some(title.to_string()),
        body: body.to_string(),
        tags: Some(vec!["rust".to_string()]),
        ..Default::default()
    }
}

#[tokio::test]
async fn save_writes_markdown_and_records_file_path() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("vault.db");
    let vault_dir = tmp.path().join("vault");
    std::fs::create_dir_all(&vault_dir).unwrap();

    let engine = SqliteBackend::new(
        db_path.to_str().unwrap(),
        vault_dir.to_str().unwrap(),
        Arc::new(NoOpEmbedder),
    )
    .expect("backend");
    engine.init().await.expect("init");

    let out = engine
        .save(insight("SQLite WAL mode avoids reader/writer contention.", "WAL mode"))
        .await
        .expect("save");

    // The row exists and points at a real markdown file. No vault.toml -> zero-config
    // default: flat folder named after the kind, not an implicit "knowledge" grouping.
    let entry = engine.get(&out.id).await.expect("get").expect("entry exists");
    let file_path = entry.file_path.expect("file_path recorded");
    assert!(file_path.contains("/insight/"), "path under kind dir: {file_path}");
    assert!(std::path::Path::new(&file_path).exists(), "the .md file was written");

    // list() must project the same 26 columns entry_from_row reads (regression:
    // LIST_COLUMNS previously omitted file_path/last_accessed_at/source_files).
    let listed = engine.list(context_vault_core::engine::ListOptions { limit: 10, ..Default::default() })
        .await
        .expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, out.id);

    // The file content carries the frontmatter + formatted body.
    let md = std::fs::read_to_string(&file_path).unwrap();
    assert!(md.starts_with("---\n"), "has frontmatter");
    assert!(md.contains(&format!("id: {}", out.id)), "frontmatter carries the id");
    assert!(md.contains("kind: insight"));
    assert!(md.contains("WAL mode avoids"), "body present");
}

#[tokio::test]
async fn reindex_rebuilds_rows_from_markdown() {
    let tmp = tempfile::tempdir().unwrap();
    let vault_dir = tmp.path().join("vault");
    std::fs::create_dir_all(&vault_dir).unwrap();

    // First backend writes two markdown files.
    let db1 = tmp.path().join("a.db");
    let e1 = SqliteBackend::new(db1.to_str().unwrap(), vault_dir.to_str().unwrap(), Arc::new(NoOpEmbedder)).unwrap();
    e1.init().await.unwrap();
    e1.save(insight("first body about caching", "First")).await.unwrap();
    e1.save(insight("second body about indexing", "Second")).await.unwrap();

    // A fresh DB pointed at the same vault, reindexed, must recover both rows from disk.
    let db2 = tmp.path().join("b.db");
    let e2 = SqliteBackend::new(db2.to_str().unwrap(), vault_dir.to_str().unwrap(), Arc::new(NoOpEmbedder)).unwrap();
    e2.init().await.unwrap();
    let summary = e2.reindex(ReindexOptions { full: true }).await.expect("reindex");
    assert_eq!(summary.entries_reindexed, 2, "both markdown files were indexed");

    let stats = e2.stats().await.unwrap();
    assert_eq!(stats.total_entries, 2);
}
