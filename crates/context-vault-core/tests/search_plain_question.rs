//! OPS-288: a question in plain words finds the entry its key words name. Before the fix,
//! FTS5 ANDed every word, so "what is our staging bucket called?" returned nothing while
//! "staging bucket" found the entry. Uses NoOpEmbedder (FTS lane only, as a fresh install).

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

fn entry(title: &str, body: &str) -> SaveInput {
    SaveInput { kind: "insight".to_string(), title: Some(title.to_string()), body: body.to_string(), ..Default::default() }
}

fn opts() -> SearchOptions {
    SearchOptions { limit: 10, ..Default::default() }
}

#[tokio::test]
async fn a_plain_question_finds_the_entry() {
    let (e, _tmp) = make().await;
    e.save(entry("staging bucket", "Our staging bucket is aurora-gravel-17.")).await.unwrap();
    e.save(entry("deploy notes", "Deploys run from main after the tests pass.")).await.unwrap();

    let hits = e.hybrid_search("what is our staging bucket called?", opts()).await.unwrap();
    assert_eq!(hits.first().and_then(|h| h.entry.title.as_deref()), Some("staging bucket"));
    assert_eq!(hits.len(), 1, "the deploy note shares no content word with the question");
}

#[tokio::test]
async fn a_question_with_one_unknown_word_falls_back_to_any_word_ranked() {
    let (e, _tmp) = make().await;
    e.save(entry("staging bucket", "Our staging bucket is aurora-gravel-17.")).await.unwrap();
    e.save(entry("production bucket", "Production lives in the prod bucket.")).await.unwrap();

    // "name" is in neither entry, so the all-words query is empty; any-word ranks the entry
    // matching both "staging" and "bucket" above the one matching "bucket" alone.
    let hits = e.hybrid_search("staging bucket name", opts()).await.unwrap();
    assert_eq!(hits.first().and_then(|h| h.entry.title.as_deref()), Some("staging bucket"));
    assert_eq!(hits.len(), 2);
}

#[tokio::test]
async fn keyword_search_keeps_all_words_precision() {
    let (e, _tmp) = make().await;
    e.save(entry("staging bucket", "Our staging bucket is aurora-gravel-17.")).await.unwrap();
    e.save(entry("production bucket", "Production lives in the prod bucket.")).await.unwrap();

    // Both words present in one entry: only that entry, as before.
    let hits = e.hybrid_search("staging bucket", opts()).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entry.title.as_deref(), Some("staging bucket"));
}

// OPS-369: with several hits, CLI search ranks the closer match higher with different scores;
// before, only the entry holding every word came back, and one hit always scored 1.6667.
async fn three_matches(e: &SqliteBackend) {
    for (title, body) in [("one", "harbor notes"), ("two", "harbor lantern notes"), ("three", "harbor lantern ferry notes")] {
        e.save(entry(title, body)).await.unwrap();
    }
}

#[tokio::test]
async fn cli_search_ranks_closer_matches_higher_with_distinct_scores() {
    let (e, _tmp) = make().await;
    three_matches(&e).await;
    let hits = e.hybrid_search("harbor lantern ferry", SearchOptions { fill_any_word: true, ..opts() }).await.unwrap();
    let titles: Vec<&str> = hits.iter().filter_map(|h| h.entry.title.as_deref()).collect();
    assert_eq!(titles, vec!["three", "two", "one"], "three words, then two, then one");
    assert!(hits[0].score > hits[1].score && hits[1].score > hits[2].score, "scores differ, best first: {:?}",
        hits.iter().map(|h| h.score).collect::<Vec<_>>());
}

#[tokio::test]
async fn recall_keeps_every_word_hits_only() {
    let (e, _tmp) = make().await;
    three_matches(&e).await;
    let hits = e.hybrid_search("harbor lantern ferry", opts()).await.unwrap();
    assert_eq!(hits.iter().filter_map(|h| h.entry.title.as_deref()).collect::<Vec<_>>(), vec!["three"]);
}

#[tokio::test]
async fn the_fill_keeps_the_search_filters() {
    let (e, _tmp) = make().await;
    three_matches(&e).await;
    e.save(SaveInput { kind: "decision".to_string(), title: Some("four".to_string()), body: "harbor lantern decision".to_string(), ..Default::default() }).await.unwrap();
    let opts = SearchOptions { fill_any_word: true, kind: Some("decision".to_string()), ..opts() };
    let hits = e.hybrid_search("harbor lantern ferry", opts).await.unwrap();
    assert_eq!(hits.iter().filter_map(|h| h.entry.title.as_deref()).collect::<Vec<_>>(), vec!["four"]);
}
