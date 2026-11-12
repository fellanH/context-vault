//! Proves SearchOptions filters (kind/tier) are applied in fts_search — previously
//! the filters were declared but ignored. Uses NoOpEmbedder (FTS lane only).

use context_vault_core::backends::{NoOpEmbedder, SqliteBackend};
use context_vault_core::engine::{SaveInput, SearchOptions, VaultEngine};
use std::sync::Arc;

async fn make() -> (SqliteBackend, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("vault.db");
    let vault = tmp.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    let e = SqliteBackend::new(db.to_str().unwrap(), vault.to_str().unwrap(), Arc::new(NoOpEmbedder)).unwrap();
    e.init().await.unwrap();
    (e, tmp)
}

fn entry(kind: &str, body: &str) -> SaveInput {
    SaveInput { kind: kind.to_string(), title: Some("Widget".to_string()), body: body.to_string(), ..Default::default() }
}

#[tokio::test]
async fn fts_search_applies_kind_filter() {
    let (e, _tmp) = make().await;
    e.save(entry("insight", "the widget spins fast")).await.unwrap();
    e.save(entry("insight", "another widget insight")).await.unwrap();
    e.save(entry("note", "a widget note")).await.unwrap();

    // No filter: all three match "widget".
    let all = e.fts_search("widget", SearchOptions { limit: 10, ..Default::default() }).await.unwrap();
    assert_eq!(all.len(), 3, "all three contain widget");

    // kind=insight filter drops the note.
    let only_insights = e
        .fts_search("widget", SearchOptions { kind: Some("insight".to_string()), limit: 10, ..Default::default() })
        .await
        .unwrap();
    assert_eq!(only_insights.len(), 2, "only the two insights survive the kind filter");
    assert!(only_insights.iter().all(|s| s.entry.kind == "insight"));
}

/// Card 19: superseded and expired rows stay out of default search;
/// `include_superseded` brings them back.
#[tokio::test]
async fn fts_search_skips_superseded_and_expired() {
    let (e, tmp) = make().await;
    let old = e.save(entry("decision", "widget old decision")).await.unwrap().id;
    let new = e.save(entry("decision", "widget new decision")).await.unwrap().id;
    let gone = e.save(entry("decision", "widget expired decision")).await.unwrap().id;
    let later = e.save(entry("decision", "widget expires later")).await.unwrap().id;

    let conn = rusqlite::Connection::open(tmp.path().join("vault.db")).unwrap();
    conn.execute("UPDATE vault SET superseded_by = ?1 WHERE id = ?2", [&new, &old]).unwrap();
    conn.execute("UPDATE vault SET expires_at = '2020-01-01T00:00:00Z' WHERE id = ?1", [&gone]).unwrap();
    conn.execute("UPDATE vault SET expires_at = '2999-01-01T00:00:00Z' WHERE id = ?1", [&later]).unwrap();

    let ids = |r: Vec<context_vault_core::engine::ScoredEntry>| {
        let mut v: Vec<String> = r.into_iter().map(|s| s.entry.id).collect();
        v.sort();
        v
    };
    let mut live = vec![new.clone(), later.clone()];
    live.sort();
    let default = e.fts_search("widget", SearchOptions { limit: 10, ..Default::default() }).await.unwrap();
    assert_eq!(ids(default), live, "only the live rows by default");

    let all = e
        .fts_search("widget", SearchOptions { include_superseded: true, limit: 10, ..Default::default() })
        .await
        .unwrap();
    assert_eq!(all.len(), 4, "include_superseded restores superseded and expired rows");
}
