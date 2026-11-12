use super::V5Backend;
use cv_core::{Registry, VaultEntry, VaultType};
use cv_engine::{ListOptions, SaveInput, SearchOptions, VaultEngine};
use std::path::Path;

fn registry(main: &Path, exhaust: &Path, default: &str) -> Registry {
    let entry = |kind, dir: &Path, search_by_default| VaultEntry { kind, dir: dir.to_string_lossy().into_owned(), search_by_default };
    let mut registry = Registry { default: default.into(), ..Default::default() };
    registry.vaults.insert("main".into(), entry(VaultType::Knowledge, main, true));
    registry.vaults.insert("exhaust".into(), entry(VaultType::Data, exhaust, false));
    registry
}

fn note(body: &str) -> SaveInput {
    SaveInput { kind: "insight".into(), title: Some("Cold start".into()), body: body.into(), tags: Some(vec!["perf".into()]), ..Default::default() }
}

fn search(limit: usize) -> SearchOptions {
    SearchOptions { limit, ..Default::default() }
}

#[tokio::test]
async fn save_search_get_list_update_delete_round_trip_on_files() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust, index) = (tmp.path().join("vault"), tmp.path().join("exhaust"), tmp.path().join("index"));
    std::fs::create_dir_all(&main).unwrap();
    std::fs::create_dir_all(&exhaust).unwrap();
    let engine = V5Backend::open(&registry(&main, &exhaust, "main"), &index).unwrap();

    let id = engine.save(note("schema init runs once per deployed version")).await.unwrap().id;
    let hits = engine.hybrid_search("schema: init (once)", search(5)).await.unwrap();
    assert_eq!(hits.len(), 1, "a save is searchable at once; query syntax never fails");
    assert_eq!(hits[0].entry.id, id);
    assert!(hits[0].entry.file_path.as_deref().is_some_and(|p| Path::new(p).starts_with(main.canonicalize().unwrap())));

    let vaults = cv_engine::situations::SituationEngine::list_vaults(&engine).await.unwrap();
    let rows: Vec<_> = vaults["vaults"].as_array().unwrap().iter().map(|v| (v["name"].as_str().unwrap(), v["records"].as_u64().unwrap(), v["default"].as_bool().unwrap())).collect();
    assert_eq!(rows, [("exhaust", 0, false), ("main", 1, true)]);
    assert!(vaults["vaults"][1]["index_bytes"].as_u64().unwrap() > 0);

    let got = engine.get(&id).await.unwrap().unwrap();
    assert_eq!((got.kind.as_str(), got.tags.as_slice()), ("insight", ["perf".to_string()].as_slice()));
    assert_eq!(engine.list(ListOptions { kind: Some("insight".into()), ..Default::default() }).await.unwrap().len(), 1);

    let mut update = note("auth check also runs once per version");
    update.id = Some(id.clone());
    assert_eq!(engine.save(update).await.unwrap().id, id);
    assert!(engine.hybrid_search("schema", search(5)).await.unwrap().is_empty(), "the old text is gone");
    assert_eq!(engine.hybrid_search("auth", search(5)).await.unwrap()[0].entry.created_at, got.created_at);

    engine.delete(&id).await.unwrap();
    assert!(engine.get(&id).await.unwrap().is_none());
    assert!(engine.hybrid_search("auth", search(5)).await.unwrap().is_empty());
    let stats = engine.stats().await.unwrap();
    assert_eq!((stats.total_entries, stats.by_kind.len()), (0, 0), "a deleted record leaves no count behind");
}

#[tokio::test]
async fn a_data_vault_default_refuses_saves() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust, index) = (tmp.path().join("vault"), tmp.path().join("exhaust"), tmp.path().join("index"));
    std::fs::create_dir_all(&main).unwrap();
    std::fs::create_dir_all(&exhaust).unwrap();
    let engine = V5Backend::open(&registry(&main, &exhaust, "exhaust"), &index).unwrap();
    let error = engine.save(note("x")).await.unwrap_err().to_string();
    assert!(error.contains("data vault"), "{error}");
}

#[tokio::test]
async fn superseded_and_expired_records_leave_default_search() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust, index) = (tmp.path().join("vault"), tmp.path().join("exhaust"), tmp.path().join("index"));
    std::fs::create_dir_all(&main).unwrap();
    std::fs::create_dir_all(&exhaust).unwrap();
    let engine = V5Backend::open(&registry(&main, &exhaust, "main"), &index).unwrap();
    let old = engine.save(note("decision gravel: use sqlite")).await.unwrap().id;
    let mut newer = note("decision gravel: use tantivy");
    newer.supersedes = Some(vec![old.clone()]);
    let new = engine.save(newer).await.unwrap().id;
    let mut lapsed = note("decision gravel: temporary freeze");
    lapsed.expires_at = Some("2026-01-01T00:00:00Z".into());
    engine.save(lapsed).await.unwrap();

    let ids = |hits: Vec<cv_engine::ScoredEntry>| hits.into_iter().map(|h| h.entry.id).collect::<Vec<_>>();
    assert_eq!(ids(engine.hybrid_search("gravel", search(10)).await.unwrap()), std::slice::from_ref(&new));
    let all = SearchOptions { include_superseded: true, ..search(10) };
    assert_eq!(engine.hybrid_search("gravel", all).await.unwrap().len(), 3);
    assert_eq!(engine.get(&old).await.unwrap().unwrap().superseded_by.as_deref(), Some(new.as_str()));
}

fn fresh() -> (tempfile::TempDir, V5Backend) {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust) = (tmp.path().join("vault"), tmp.path().join("exhaust"));
    std::fs::create_dir_all(&main).unwrap();
    std::fs::create_dir_all(&exhaust).unwrap();
    let engine = V5Backend::open(&registry(&main, &exhaust, "main"), &tmp.path().join("index")).unwrap();
    (tmp, engine)
}

#[tokio::test]
async fn an_update_keeps_supersession_and_a_record_never_supersedes_itself() {
    let (_tmp, engine) = fresh();
    let old = engine.save(note("pebble plan one")).await.unwrap().id;
    let mut newer = note("pebble plan two");
    newer.supersedes = Some(vec![old.clone()]);
    let new = engine.save(newer).await.unwrap().id;
    let mut retag = note("pebble plan two, retagged");
    retag.id = Some(new.clone());
    engine.save(retag).await.unwrap();
    assert_eq!(engine.get(&old).await.unwrap().unwrap().superseded_by.as_deref(), Some(new.as_str()), "update without supersedes kept it");

    let mut selfish = note("pebble plan two, again");
    selfish.id = Some(new.clone());
    selfish.supersedes = Some(vec![new.clone()]);
    engine.save(selfish).await.unwrap();
    assert!(engine.get(&new).await.unwrap().unwrap().superseded_by.is_none());
    assert_eq!(engine.hybrid_search("pebble", search(5)).await.unwrap().len(), 1);
}

#[tokio::test]
async fn filtered_pages_are_full_and_offsets_count_only_matches() {
    let (_tmp, engine) = fresh();
    let mut stale = Vec::new();
    for n in 0..6 {
        stale.push(engine.save(note(&format!("cobble cobble cobble draft {n}"))).await.unwrap().id);
    }
    let mut live = note("cobble final");
    live.supersedes = Some(stale.clone());
    let live = engine.save(live).await.unwrap().id;
    let page = engine.hybrid_search("cobble", search(1)).await.unwrap();
    assert_eq!(page.iter().map(|h| h.entry.id.clone()).collect::<Vec<_>>(), std::slice::from_ref(&live), "superseded top hits don't empty the page");

    let decision = |body: &str| SaveInput { kind: "decision".into(), body: body.into(), ..Default::default() };
    engine.save(decision("d1")).await.unwrap();
    engine.save(decision("d2")).await.unwrap();
    let category = engine.get(&live).await.unwrap().unwrap().category;
    let insights = |offset| ListOptions { category: Some(category.clone()), limit: 3, offset, ..Default::default() };
    let all = engine.list(ListOptions { category: Some(category.clone()), limit: 100, ..Default::default() }).await.unwrap().len();
    assert_eq!(all, 9, "7 insights + 2 decisions share the knowledge category");
    assert_eq!(engine.list(insights(0)).await.unwrap().len(), 3);
    assert_eq!(engine.list(insights(7)).await.unwrap().len(), 2, "offset 7 of 9 matches leaves two");
    let only_decisions = ListOptions { kind: Some("decision".into()), limit: 1, offset: 1, ..Default::default() };
    assert_eq!(engine.list(only_decisions).await.unwrap().len(), 1);
}

#[tokio::test]
async fn situations_list_by_size_and_page_in_order_on_v5() {
    use cv_engine::situations::{SituationEngine, PAGE_CHARS};
    let (_tmp, engine) = fresh();
    let with = |body: String, situations: &[&str], order: i64| SaveInput {
        meta: Some(serde_json::json!({ "situations": situations, "situation_order": order })),
        ..note(&body)
    };
    let big = "x".repeat(PAGE_CHARS * 2 / 3);
    let first = engine.save(with(format!("first {big}"), &["before-deploy"], 1)).await.unwrap().id;
    let second = engine.save(with(format!("second {big}"), &["before-deploy", "fresh"], 2)).await.unwrap().id;
    let mut gone = with("expired".into(), &["fresh"], 1);
    gone.expires_at = Some("2026-01-01T00:00:00Z".into());
    engine.save(gone).await.unwrap();

    let listed = engine.list_situations().await.unwrap();
    assert_eq!(listed["situations"], serde_json::json!([{"situation": "before-deploy", "entries": 2}, {"situation": "fresh", "entries": 1}]));
    let page = engine.read_situation("before-deploy", None).await.unwrap();
    assert_eq!(page["entries"].as_array().unwrap().len(), 1, "two 2/3-page bodies don't fit one page");
    assert_eq!(page["entries"][0]["id"], first.as_str());
    let next = engine.read_situation("before-deploy", page["next_cursor"].as_str()).await.unwrap();
    assert_eq!(next["entries"][0]["id"], second.as_str());
    assert!(next["next_cursor"].is_null());
    assert!(engine.read_situation("Not A Name", None).await.is_err());

    let map = engine.vault_map(false).await.unwrap();
    let kinds = &map["areas"][0]["kinds"][0];
    assert_eq!((kinds["kind"].as_str(), kinds["entries"].as_u64()), (Some("insight"), Some(3)));
    let keys = kinds["key_entries"].as_array().unwrap();
    assert!(keys.iter().all(|k| k["situations"].is_array() || k["title"] == "Cold start"), "situated entries carry their situations");
    assert_eq!(map["tags"][0]["tag"], "perf");
    assert_eq!(map["situations"][0]["situation"], "before-deploy");
}
