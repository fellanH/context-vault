use super::handlers_situations::{handle_context_for, handle_list_situations, handle_vault_map};
use crate::engine::VaultEngine;
use std::sync::Arc;
use serde_json::Value;
use crate::backends::{NoOpEmbedder, SqliteBackend};
use serde_json::json;
use super::handlers_write::handle_save_context;

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

fn ids(text: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(text).unwrap();
    v["entries"].as_array().unwrap().iter().map(|e| e["id"].as_str().unwrap().to_string()).collect()
}

#[tokio::test]
async fn save_with_situations_then_context_for_and_map() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = engine(&tmp).await;
    let a = id_of(&handle_save_context(engine.clone(), &json!({"kind": "insight", "title": "Deploy checklist", "body": "Run the smoke test first. Then ship.", "situations": ["before-deploy", "fresh"], "situation_order": 5})).await.unwrap());
    let b = id_of(&handle_save_context(engine.clone(), &json!({"kind": "insight", "title": "Other", "body": "second", "situations": ["fresh"]})).await.unwrap());
    let _plain = handle_save_context(engine.clone(), &json!({"kind": "insight", "body": "unmarked"})).await.unwrap();

    // the markdown carries them, so a reindex rebuilds the same index
    let md = std::fs::read_to_string(engine.get(&a).await.unwrap().unwrap().file_path.unwrap()).unwrap();
    assert!(md.contains(r#"situations: ["before-deploy", "fresh"]"#) && md.contains("situation_order: 5"), "{md}");

    let page = handle_context_for(engine.clone(), &json!({"situation": "fresh"})).await.unwrap();
    assert_eq!(ids(&page), vec![a.clone(), b.clone()]);
    let listed = handle_list_situations(engine.clone()).await.unwrap();
    assert!(listed.contains("before-deploy") && listed.contains("fresh"));

    // update by id: order alone re-orders; null clears; a new meta keeps them
    handle_save_context(engine.clone(), &json!({"id": b, "situation_order": 1})).await.unwrap();
    assert_eq!(ids(&handle_context_for(engine.clone(), &json!({"situation": "fresh"})).await.unwrap()), vec![b.clone(), a.clone()]);
    handle_save_context(engine.clone(), &json!({"id": a, "meta": {"note": 1}})).await.unwrap();
    assert_eq!(ids(&handle_context_for(engine.clone(), &json!({"situation": "before-deploy"})).await.unwrap()), vec![a.clone()]);
    handle_save_context(engine.clone(), &json!({"id": a, "situations": null})).await.unwrap();
    assert!(ids(&handle_context_for(engine.clone(), &json!({"situation": "before-deploy"})).await.unwrap()).is_empty());

    // reindex from the markdown preserves what is still there
    engine.reindex(crate::engine::ReindexOptions { full: true }).await.unwrap();
    assert_eq!(ids(&handle_context_for(engine.clone(), &json!({"situation": "fresh"})).await.unwrap()), vec![b.clone()]);

    assert!(handle_save_context(engine.clone(), &json!({"kind": "insight", "body": "x", "situations": ["Bad Name"]})).await.is_err());
    assert!(handle_context_for(engine.clone(), &json!({"situation": "fresh", "cursor": "nope"})).await.is_err());

    let map: Value = serde_json::from_str(&handle_vault_map(engine.clone(), &json!({})).await.unwrap()).unwrap();
    assert_eq!(map["totals"]["live"], 3);
    assert_eq!(map["areas"][0]["kinds"][0]["key_entries"][0]["id"], b);
}

async fn saved(engine: &Arc<dyn VaultEngine>) -> (String, String) {
    let reply = handle_save_context(engine.clone(), &json!({"kind": "insight", "body": "first", "situations": ["fresh"]})).await.unwrap();
    let id = id_of(&reply);
    let path = engine.get(&id).await.unwrap().unwrap().file_path.unwrap();
    (id, path)
}

async fn fresh_ids(engine: &Arc<dyn VaultEngine>) -> Vec<String> {
    ids(&handle_context_for(engine.clone(), &json!({"situation": "fresh"})).await.unwrap())
}

#[tokio::test]
async fn hand_edited_malformed_situations_stay_editable() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = engine(&tmp).await;
    let (id, path) = saved(&engine).await;
    let md = std::fs::read_to_string(&path).unwrap().replace(r#"situations: ["fresh"]"#, "situations: Before Deploy");
    std::fs::write(&path, md).unwrap();
    engine.reindex(crate::engine::ReindexOptions { full: true }).await.unwrap();
    assert!(fresh_ids(&engine).await.is_empty(), "a malformed value is not indexed");

    handle_save_context(engine.clone(), &json!({"id": id, "body": "second"})).await.unwrap();
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("second") && after.contains("situations: Before Deploy"), "{after}");
}

#[tokio::test]
async fn reindex_drops_situations_removed_from_the_file() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = engine(&tmp).await;
    let (id, path) = saved(&engine).await;
    assert_eq!(fresh_ids(&engine).await, vec![id]);
    let md = std::fs::read_to_string(&path).unwrap().replace(r#"situations: ["fresh"]"#, "");
    std::fs::write(&path, md).unwrap();
    engine.reindex(crate::engine::ReindexOptions { full: true }).await.unwrap();
    assert!(fresh_ids(&engine).await.is_empty());
}

#[tokio::test]
async fn deleting_an_entry_removes_it_from_its_situations() {
    let tmp = tempfile::tempdir().unwrap();
    let engine = engine(&tmp).await;
    let (id, _) = saved(&engine).await;
    engine.delete(&id).await.unwrap();
    assert!(fresh_ids(&engine).await.is_empty());
}
