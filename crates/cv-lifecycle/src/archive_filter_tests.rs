//! OPS-406: archive by filter counts first, moves only what matches, verifies every byte
//! against its manifest, and the manifest brings every file back.

use crate::archive_filter::{archive_by_filter, select, ArchiveFilter};
use crate::archive_manifest::{fnv64, restore_by_manifest};
use std::fs;

fn vault() -> (tempfile::TempDir, rusqlite::Connection) {
    let tmp = tempfile::tempdir().unwrap();
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE vault (id TEXT, kind TEXT, category TEXT, title TEXT, tier TEXT, file_path TEXT, created_at TEXT, updated_at TEXT);").unwrap();
    let rows = [
        ("e1", "event", "events/event/e1.md", "2026-05-03T10:00:00Z"),
        ("e2", "event", "events/event/e2.md", "2026-10-01T10:00:00Z"),
        ("s1", "session", "events/session/s1.md", "2026-06-01T10:00:00Z"),
        ("i1", "insight", "knowledge/insight/i1.md", "2026-05-04T10:00:00Z"),
    ];
    for (id, kind, rel, created) in rows {
        let path = tmp.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, format!("---\nid: {id}\n---\nbody of {id}\n")).unwrap();
        db.execute("INSERT INTO vault VALUES (?1, ?2, NULL, ?1, 'working', ?3, ?4, NULL)", rusqlite::params![id, kind, path.to_str().unwrap(), created]).unwrap();
    }
    db.execute("INSERT INTO vault VALUES ('nofile', 'event', NULL, 'x', 'working', NULL, '2026-05-01T00:00:00Z', NULL)", []).unwrap();
    (tmp, db)
}

fn ids(db: &rusqlite::Connection) -> Vec<String> {
    let mut s = db.prepare("SELECT id FROM vault ORDER BY id").unwrap();
    s.query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
}

#[test]
fn archive_by_filter_moves_what_matches_verifies_it_and_restores_it() {
    let (tmp, db) = vault();
    let dir = tmp.path().to_str().unwrap();
    let f = ArchiveFilter { kinds: vec!["event".into(), "session".into()], before: Some("2026-07-01".into()), ..Default::default() };

    let dry = archive_by_filter(&db, dir, &f, false).unwrap();
    assert_eq!((dry.matched, dry.no_file, dry.moved), (3, 1, 0));
    assert_eq!(ids(&db).len(), 5, "a dry run moves nothing");

    let done = archive_by_filter(&db, dir, &f, true).unwrap();
    assert_eq!((done.moved, done.verified), (2, 2));
    assert_eq!(ids(&db), vec!["e2", "i1", "nofile"], "e2 is newer, i1 another kind, nofile has no file");
    assert!(tmp.path().join("_archive/events/event/e1.md").exists() && !tmp.path().join("events/event/e1.md").exists());

    let manifest = done.manifest.unwrap();
    assert_eq!(restore_by_manifest(dir, &manifest, false).unwrap(), (2, 0, 0));
    assert!(tmp.path().join("_archive/events/event/e1.md").exists(), "a restore dry run moves nothing");
    assert_eq!(restore_by_manifest(dir, &manifest, true).unwrap(), (2, 2, 0));
    assert_eq!(fs::read_to_string(tmp.path().join("events/session/s1.md")).unwrap(), "---\nid: s1\n---\nbody of s1\n");
    assert_eq!(restore_by_manifest(dir, &manifest, true).unwrap(), (0, 0, 2), "a second restore finds them in place");
}

#[test]
fn an_empty_filter_is_refused_and_the_hash_is_stable() {
    let (_tmp, db) = vault();
    assert!(select(&db, &ArchiveFilter::default()).unwrap_err().to_string().contains("whole vault"));
    assert_eq!(fnv64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv64(b"a"), 0xaf63_dc4c_8601_ec8c);
}
