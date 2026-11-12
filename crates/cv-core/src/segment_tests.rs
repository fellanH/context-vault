use std::io::Write;

use chrono::{Duration, Utc};

use crate::record::Record;
use crate::segment_read::{read_at, read_segment, segment_files};
use crate::segment_append::{append, append_batch};
use crate::segment_write::seal_before;

fn rec(text: &str) -> Record {
    let mut r = Record::new("session", text);
    r.meta.insert("n".into(), serde_json::json!(text.len()));
    r
}

fn all(dir: &std::path::Path) -> (Vec<Record>, u64) {
    let (mut out, mut bad) = (Vec::new(), 0);
    for seg in segment_files(dir).unwrap() {
        bad += read_segment(&seg, 0, |_, r| {
            out.push(r);
            Ok(())
        })
        .unwrap();
    }
    (out, bad)
}

#[test]
fn append_then_read_back_by_location_and_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let recs: Vec<Record> = ["alpha", "bravo", "charlie"].iter().map(|t| rec(t)).collect();
    let locs: Vec<_> = recs.iter().map(|r| append(tmp.path(), "session", r).unwrap()).collect();
    assert_eq!(locs[0].offset, 0);
    assert_eq!(locs[1].offset, locs[0].len);
    for (r, l) in recs.iter().zip(&locs) {
        assert_eq!(&read_at(tmp.path(), l).unwrap(), r);
    }
    assert_eq!(all(tmp.path()), (recs, 0));
}

#[test]
fn concurrent_appends_never_tear_or_collide() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let handles: Vec<_> = (0..8)
        .map(|t| {
            let dir = dir.clone();
            std::thread::spawn(move || (0..50).map(|i| append(&dir, "tool-call", &rec(&format!("t{t}-{i}-{}", "x".repeat(i * 97)))).unwrap()).collect::<Vec<_>>())
        })
        .collect();
    let mut offsets: Vec<u64> = handles.into_iter().flat_map(|h| h.join().unwrap()).map(|l| l.offset).collect();
    offsets.sort();
    offsets.dedup();
    assert_eq!(offsets.len(), 400, "every append got its own offset");
    let (read, bad) = all(&dir);
    assert_eq!((read.len(), bad), (400, 0));
}

#[test]
fn sealing_keeps_locations_valid_and_is_plain_zstd() {
    let tmp = tempfile::tempdir().unwrap();
    let mut old = rec("yesterday's record");
    old.created = Utc::now() - Duration::days(1);
    let mut first = rec("first of that day, so the next one sits at a non-zero offset");
    first.created = old.created;
    append(tmp.path(), "session", &first).unwrap();
    let loc = append(tmp.path(), "session", &old).unwrap();
    assert!(loc.offset > 0, "the checked record must not be at offset 0");
    let today_loc = append(tmp.path(), "session", &rec("today")).unwrap();

    let sealed = seal_before(tmp.path(), Utc::now().date_naive()).unwrap();
    assert_eq!(sealed.len(), 1, "only the past day is sealed");
    assert!(sealed[0].to_string_lossy().ends_with(".0.jsonl.zst"));
    assert_eq!(read_at(tmp.path(), &loc).unwrap(), old, "same Loc after sealing");
    assert_eq!(read_at(tmp.path(), &today_loc).unwrap().text, "today");

    // Readable without the tool.
    let plain = zstd::decode_all(std::fs::File::open(&sealed[0]).unwrap()).unwrap();
    assert!(String::from_utf8(plain).unwrap().contains("yesterday's record"));

    // A late record for the sealed day goes to a new part, not into the sealed one.
    let mut late = rec("late import");
    late.created = old.created;
    assert_eq!(append(tmp.path(), "session", &late).unwrap().part, 1);
    assert_eq!(all(tmp.path()).0.len(), 4);
}

#[test]
fn torn_line_is_skipped_and_counted() {
    let tmp = tempfile::tempdir().unwrap();
    append(tmp.path(), "log", &rec("whole")).unwrap();
    let seg = &segment_files(tmp.path()).unwrap()[0];
    std::fs::OpenOptions::new().append(true).open(&seg.path).unwrap().write_all(b"{\"id\":\"torn").unwrap();
    let (read, bad) = all(tmp.path());
    assert_eq!((read.len(), bad), (1, 1));
}

#[test]
fn back_dated_records_land_in_their_own_day() {
    let tmp = tempfile::tempdir().unwrap();
    let mut r = rec("from 2026-03-01");
    r.created = "2026-03-01T10:00:00Z".parse().unwrap();
    let loc = append(tmp.path(), "import", &r).unwrap();
    assert_eq!(loc.day.to_string(), "2026-03-01");
    assert!(segment_files(tmp.path()).unwrap()[0].path.ends_with("segments/import/2026-03/2026-03-01.0.jsonl"));
}

#[test]
fn refuses_unsafe_stream_names() {
    let tmp = tempfile::tempdir().unwrap();
    for bad in ["", "../x", "a/b", ".hidden"] {
        assert!(append(tmp.path(), bad, &rec("x")).is_err(), "{bad:?} accepted");
    }
}

#[test]
fn data_segments_enforce_the_same_policy_before_creating_files() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("vault.toml"), "schema_version = 1\n[vault]\nclosed_kinds = true\npersonal_free = true\n[kinds.session.fields.count]\ntype = 'integer'\nrequired = true\n").unwrap();
    let mut r = rec("policy control");
    r.meta.insert("count".into(), serde_json::json!(1));
    r.meta.insert("sensitivity".into(), serde_json::json!("personal"));
    assert!(append(tmp.path(), "session", &r).is_err());
    assert!(!tmp.path().join("segments").exists());
    r.meta.insert("sensitivity".into(), serde_json::json!("public"));
    r.meta.insert("count".into(), serde_json::json!("one"));
    assert!(append(tmp.path(), "session", &r).is_err());
    r.meta.insert("count".into(), serde_json::json!(1));
    r.kind = "sytem".into();
    assert!(append(tmp.path(), "session", &r).is_err());
    r.kind = "session".into();
    append(tmp.path(), "session", &r).unwrap();
    assert_eq!(all(tmp.path()).0.len(), 1);
}

#[test]
fn a_batch_lands_per_day_in_input_order_and_is_all_or_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let mut recs: Vec<Record> = ["b1", "a1", "b2"].iter().map(|t| rec(t)).collect();
    for (r, day) in recs.iter_mut().zip(["2026-03-02", "2026-03-01", "2026-03-02"]) {
        r.created = format!("{day}T10:00:00Z").parse().unwrap();
    }
    let locs = append_batch(tmp.path(), "import", &recs).unwrap();
    for (loc, r) in locs.iter().zip(&recs) {
        assert_eq!(&read_at(tmp.path(), loc).unwrap(), r);
    }
    assert_eq!((locs[0].day, locs[2].offset), (locs[2].day, locs[0].len), "b2 follows b1 in one file");
    assert_eq!(segment_files(tmp.path()).unwrap().len(), 2);
    std::fs::write(tmp.path().join("vault.toml"), "schema_version = 1\n[vault]\npersonal_free = true\n").unwrap();
    let mut bad = recs.clone();
    bad[1].meta.insert("sensitivity".into(), serde_json::json!("personal"));
    assert!(append_batch(tmp.path(), "import", &bad).is_err());
    assert_eq!(all(tmp.path()).0.len(), 3, "a refused record writes nothing from its batch");
}
