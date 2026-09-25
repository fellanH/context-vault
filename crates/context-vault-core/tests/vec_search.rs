//! End-to-end proof that the vec0 lane stores embeddings on save and ranks
//! them by KNN distance on search. Uses a deterministic in-test embedder so the
//! nearest neighbour is known up front — no model download, no network.

use async_trait::async_trait;
use context_vault_core::backends::{NoOpEmbedder, SqliteBackend};
use context_vault_core::engine::{Embedder, SaveInput, SearchOptions, VaultEngine};
use std::sync::Arc;

/// Maps a handful of keywords to fixed 4-d vectors. "cat" and "car" are close;
/// "dog" is far. Anything else is the zero vector (treated as signal-free).
struct KeywordEmbedder;

#[async_trait]
impl Embedder for KeywordEmbedder {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let t = text.to_lowercase();
        let v = if t.contains("cat") {
            [1.0, 0.0, 0.0, 0.0]
        } else if t.contains("car") {
            [0.9, 0.1, 0.0, 0.0]
        } else if t.contains("dog") {
            [0.0, 1.0, 0.0, 0.0]
        } else {
            [0.0, 0.0, 0.0, 0.0]
        };
        Ok(v.to_vec())
    }

    fn dim(&self) -> usize {
        4
    }
}

async fn save_body(engine: &SqliteBackend, body: &str) {
    engine
        .save(SaveInput {
            kind: "note".to_string(),
            body: body.to_string(),
            ..Default::default()
        })
        .await
        .expect("save");
}

#[tokio::test]
async fn vec_search_ranks_nearest_neighbour_first() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("vault.db");
    let engine = SqliteBackend::new(
        db_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        Arc::new(KeywordEmbedder),
    )
    .expect("backend");
    engine.init().await.expect("init");

    save_body(&engine, "a fluffy cat sat on the mat").await;
    save_body(&engine, "a loyal dog ran in the park").await;
    save_body(&engine, "a fast car drove down the road").await;

    // Query embeds to the "cat" vector; nearest is the cat note, then car, then dog.
    let query_vec = [1.0_f32, 0.0, 0.0, 0.0];
    let hits = engine
        .vec_search(&query_vec, SearchOptions { limit: 10, ..Default::default() })
        .await
        .expect("vec_search");

    assert_eq!(hits.len(), 3, "all three embedded rows are returned");
    assert!(hits[0].entry.body.contains("cat"), "cat ranks first, got: {}", hits[0].entry.body);
    assert!(hits[1].entry.body.contains("car"), "car ranks second, got: {}", hits[1].entry.body);
    assert!(hits[2].entry.body.contains("dog"), "dog ranks last, got: {}", hits[2].entry.body);

    // Distances must be sorted ascending and scores descending.
    assert!(hits[0].vec_rank.unwrap() <= hits[1].vec_rank.unwrap());
    assert!(hits[1].vec_rank.unwrap() <= hits[2].vec_rank.unwrap());
    assert!(hits[0].score >= hits[1].score);
}

#[tokio::test]
async fn noop_embedder_leaves_vec_lane_empty() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("vault.db");
    let engine = SqliteBackend::new(
        db_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        Arc::new(NoOpEmbedder),
    )
    .expect("backend");
    engine.init().await.expect("init");

    save_body(&engine, "this entry has no real embedding").await;

    // A signal-free query returns nothing, and hybrid_search degrades to FTS.
    let vec_hits = engine
        .vec_search(&vec![0.0; 384], SearchOptions { limit: 10, ..Default::default() })
        .await
        .expect("vec_search");
    assert!(vec_hits.is_empty(), "zero embeddings are not ranked");

    let hybrid = engine
        .hybrid_search("embedding", SearchOptions { limit: 10, ..Default::default() })
        .await
        .expect("hybrid_search");
    assert!(!hybrid.is_empty(), "FTS still finds the entry with NoOp embedder");
}
