use crate::engine::V5Backend;
use cv_core::{Registry, VaultEntry, VaultType};
use cv_engine::{SaveInput, SearchOptions, VaultEngine};

#[tokio::test]
async fn machine_records_saved_on_v5_go_to_the_data_vault_and_stay_findable() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust) = (tmp.path().join("vault"), tmp.path().join("exhaust"));
    std::fs::create_dir_all(&main).unwrap();
    std::fs::create_dir_all(&exhaust).unwrap();
    std::fs::write(main.join("vault.toml"), "schema_version = 1\n[kinds.session]\ncategory = \"events\"\ntier = \"ephemeral\"\n").unwrap();
    let entry = |kind, dir: &std::path::Path, search_by_default| VaultEntry { kind, dir: dir.to_string_lossy().into_owned(), search_by_default };
    let mut registry = Registry { default: "main".into(), ..Default::default() };
    registry.vaults.insert("main".into(), entry(VaultType::Knowledge, &main, true));
    registry.vaults.insert("exhaust".into(), entry(VaultType::Data, &exhaust, false));
    let engine = V5Backend::open(&registry, &tmp.path().join("index")).unwrap();

    let session = SaveInput { kind: "session".into(), title: Some("session summary".into()), body: "quasarfrog work today".into(), ..Default::default() };
    let id = engine.save(session).await.unwrap().id;
    assert!(!main.join("events").exists(), "nothing written into the pruned events/ folder");
    let got = engine.get(&id).await.unwrap().expect("found by id in the data vault");
    assert_eq!((got.kind.as_str(), got.body.as_str()), ("session", "quasarfrog work today"));
    let default = engine.hybrid_search("quasarfrog", SearchOptions { limit: 5, ..Default::default() }).await.unwrap();
    assert!(default.is_empty(), "machine records stay out of default search");

    let note = SaveInput { kind: "insight".into(), body: "quasarfrog insight".into(), ..Default::default() };
    engine.save(note).await.unwrap();
    assert_eq!(engine.hybrid_search("quasarfrog", SearchOptions { limit: 5, ..Default::default() }).await.unwrap().len(), 1);
}
