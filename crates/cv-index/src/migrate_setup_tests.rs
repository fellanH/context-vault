use super::{setup, Targets};
use crate::migrate::MigrateOptions;
use std::path::Path;

fn note(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

#[test]
fn a_failed_run_leaves_v4_in_place_and_a_retry_completes() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust) = (tmp.path().join("main"), tmp.path().join("exhaust"));
    note(&main, "events/session/s.md", "---\nid: s1\nkind: session\ncreated: 2026-05-02T09:00:00Z\n---\nsession\n");
    note(&main, "knowledge/insight/a.md", "---\nid: dup\nkind: insight\n---\na\n");
    note(&main, "knowledge/insight/b.md", "---\nid: dup\nkind: insight\n---\nb\n");
    std::fs::create_dir_all(&exhaust).unwrap();
    let targets = Targets { registry: tmp.path().join("data/vaults.toml"), index_root: tmp.path().join("data/index") };
    let options = || MigrateOptions { main: main.clone(), exhaust: exhaust.clone(), chat_logs: false, legacy_ids: Default::default() };

    assert!(setup(&options(), &targets).is_err(), "duplicate knowledge ids refuse the main index");
    assert!(!targets.registry.exists(), "no registry: v4 stays in use");

    std::fs::remove_file(main.join("knowledge/insight/b.md")).unwrap();
    let report = setup(&options(), &targets).expect("the retry clears its own segments and completes");
    assert_eq!((report.main_indexed, report.exhaust_indexed), (1, 1));
    assert!(targets.registry.exists() && !exhaust.join(".migrating").exists());
    assert!(setup(&options(), &targets).is_err(), "a migrated machine refuses a second run");
}

#[test]
fn segments_someone_else_wrote_are_never_cleared() {
    let tmp = tempfile::tempdir().unwrap();
    let (main, exhaust) = (tmp.path().join("main"), tmp.path().join("exhaust"));
    note(&main, "knowledge/insight/a.md", "---\nid: a\nkind: insight\n---\na\n");
    note(&exhaust, "segments/session/2026-05/2026-05-02.jsonl", "{}\n");
    let targets = Targets { registry: tmp.path().join("data/vaults.toml"), index_root: tmp.path().join("data/index") };
    let options = MigrateOptions { main, exhaust: exhaust.clone(), chat_logs: false, legacy_ids: Default::default() };
    for _ in 0..2 {
        assert!(setup(&options, &targets).is_err());
    }
    assert!(exhaust.join("segments/session/2026-05/2026-05-02.jsonl").exists() && !exhaust.join(".migrating").exists());
}
