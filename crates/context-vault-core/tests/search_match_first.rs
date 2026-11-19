//! OPS-468: a closer match beats a newer weak one. Hits holding every word of the query print
//! before hits holding only some, however old; tier, heat and recency order hits inside one of
//! those blocks. Recall searches the same way (any-word fill on since OPS-402), then puts hints
//! whose titles hold the prompt's words first (recall_select.rs), so its order is this one
//! re-sorted by title overlap. NoOpEmbedder: the full-text lane only, as the CLI and the hook run.

use context_vault_core::backends::{NoOpEmbedder, SqliteBackend};
use context_vault_core::engine::{SaveInput, SearchOptions, VaultEngine};
use std::sync::Arc;

async fn make() -> (SqliteBackend, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    let e = SqliteBackend::new(tmp.path().join("vault.db").to_str().unwrap(), vault.to_str().unwrap(), Arc::new(NoOpEmbedder)).unwrap();
    e.init().await.unwrap();
    (e, tmp)
}

async fn save(e: &SqliteBackend, title: &str, body: &str, tier: &str, days_old: i64) {
    let input = SaveInput { kind: "insight".into(), title: Some(title.into()), body: body.into(), tier: Some(tier.into()), ..Default::default() };
    let id = e.save(input).await.unwrap().id;
    let when = (chrono::Utc::now() - chrono::Duration::days(days_old)).to_rfc3339();
    e.raw_db().lock().unwrap().execute("UPDATE vault SET updated_at = ?1 WHERE id = ?2", rusqlite::params![when, id]).unwrap();
}

fn titles(hits: &[context_vault_core::engine::ScoredEntry]) -> Vec<&str> {
    hits.iter().filter_map(|h| h.entry.title.as_deref()).collect()
}

#[tokio::test]
async fn a_60_day_old_exact_match_prints_before_a_fresh_one_word_hit() {
    let (e, _tmp) = make().await;
    save(&e, "old exact", "harbor lantern ferry schedule", "ephemeral", 60).await;
    save(&e, "fresh weak", "harbor gossip", "durable", 0).await;
    let cli = SearchOptions { limit: 10, fill_any_word: true, ..Default::default() };
    let hits = e.hybrid_search("harbor lantern ferry", cli).await.unwrap();
    assert_eq!(titles(&hits), vec!["old exact", "fresh weak"]);
}

#[tokio::test]
async fn tier_and_recency_still_order_hits_of_equal_match_strength() {
    let (e, _tmp) = make().await;
    save(&e, "old ephemeral", "harbor lantern ferry", "ephemeral", 60).await;
    save(&e, "fresh durable", "harbor lantern ferry", "durable", 0).await;
    save(&e, "one word", "lantern", "durable", 0).await;
    let cli = SearchOptions { limit: 10, fill_any_word: true, ..Default::default() };
    let hits = e.hybrid_search("harbor lantern ferry", cli).await.unwrap();
    assert_eq!(titles(&hits), vec!["fresh durable", "old ephemeral", "one word"]);
}
