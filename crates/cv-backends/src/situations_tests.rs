use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::situations::{fill_page, list_situations, read_situation, Key};

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch(include_str!("../../../fixtures/bench/reference_schema.sql")).unwrap();
    c
}

fn add(c: &Connection, id: &str, meta: Option<&str>, body: &str, updated: &str, expires: Option<&str>) {
    c.execute(
        "INSERT INTO vault (id, kind, title, body, meta, created_at, updated_at, expires_at) VALUES (?1,'insight',?1,?2,?3,?4,?4,?5)",
        params![id, body, meta, updated, expires],
    )
    .unwrap();
}

fn ids(page: &Value) -> Vec<String> {
    page["entries"].as_array().unwrap().iter().map(|e| e["id"].as_str().unwrap().to_string()).collect()
}

fn seeded() -> Connection {
    let c = conn();
    add(&c, "a", Some(r#"{"situations":["fresh","debugging"],"situation_order":10}"#), "A", "2026-01-01T00:00:00Z", None);
    add(&c, "b", Some(r#"{"situations":["fresh"]}"#), "B", "2026-01-02T00:00:00Z", None);
    add(&c, "c", Some(r#"{"situations":["fresh"]}"#), "C", "2026-01-03T00:00:00Z", None);
    add(&c, "gone", Some(r#"{"situations":["fresh"]}"#), "X", "2026-01-04T00:00:00Z", Some("2001-01-01T00:00:00Z"));
    add(&c, "plain", None, "P", "2026-01-05T00:00:00Z", None);
    add(&c, "broken", Some("{not json situations"), "Z", "2026-01-05T00:00:00Z", None);
    c
}

#[test]
fn order_and_expiry() {
    let c = seeded();
    // order asc, then updated desc; expired and unmarked entries never appear.
    let page = read_situation(&c, "fresh", None, 12_000).unwrap();
    assert_eq!(ids(&page), vec!["a", "c", "b"]);
    assert_eq!(page["entries"][0]["situation_order"], 10);
    assert_eq!(page["entries"][1]["situation_order"], 100);
    assert!(page["next_cursor"].is_null());
    let listed = list_situations(&c).unwrap()["situations"].clone();
    assert_eq!(listed, json!([{"situation":"fresh","entries":3},{"situation":"debugging","entries":1}]));
}

#[test]
fn triggers_follow_edits_and_deletes() {
    let c = seeded();
    c.execute("UPDATE vault SET meta = ?1 WHERE id = 'b'", [r#"{"situations":["debugging"]}"#]).unwrap();
    c.execute("UPDATE vault SET meta = ?1 WHERE id = 'broken'", ["not json"]).unwrap();
    c.execute("DELETE FROM vault WHERE id = 'c'", []).unwrap();
    assert_eq!(ids(&read_situation(&c, "fresh", None, 12_000).unwrap()), vec!["a"]);
    assert_eq!(ids(&read_situation(&c, "debugging", None, 12_000).unwrap()), vec!["a", "b"]);
    // a write that does not touch meta/updated_at leaves the index alone
    c.execute("UPDATE vault SET hit_count = 5 WHERE id = 'a'", []).unwrap();
    assert_eq!(ids(&read_situation(&c, "fresh", None, 12_000).unwrap()), vec!["a"]);
}

fn five_long() -> Connection {
    let c = conn();
    for i in 0..5 {
        let updated = format!("2026-01-0{}T00:00:00Z", i + 1);
        add(&c, &format!("e{i}"), Some(r#"{"situations":["post-compaction"]}"#), &"x".repeat(100), &updated, None);
    }
    c
}

#[test]
fn pages_by_chars_with_a_cursor() {
    let c = five_long();
    let p1 = read_situation(&c, "post-compaction", None, 250).unwrap();
    assert_eq!(ids(&p1), vec!["e4", "e3"]);
    let p2 = read_situation(&c, "post-compaction", p1["next_cursor"].as_str(), 250).unwrap();
    assert_eq!(ids(&p2), vec!["e2", "e1"]);
    let p3 = read_situation(&c, "post-compaction", p2["next_cursor"].as_str(), 250).unwrap();
    assert_eq!(ids(&p3), vec!["e0"]);
    assert!(p3["next_cursor"].is_null());
}

#[test]
fn long_body_is_cut_and_bad_input_refused() {
    let c = five_long();
    let cut = read_situation(&c, "post-compaction", None, 40).unwrap();
    assert_eq!(cut["entries"][0]["truncated"], true);
    assert_eq!(cut["entries"][0]["body_length"], 100);
    assert_eq!(cut["entries"][0]["body"].as_str().unwrap().len(), 40);
    assert!(read_situation(&c, "post-compaction", Some("zz"), 250).is_err());
    assert!(read_situation(&c, "Not Valid", None, 250).is_err());
}

#[test]
fn an_entry_gone_between_the_two_reads_does_not_end_paging() {
    let c = five_long();
    let key = |id: &str| Key { id: id.into(), ord: 100, updated: String::new() };
    let (entries, consumed) = fill_page(&c, &[key("e4"), key("vanished")], 1000).unwrap();
    assert_eq!((entries.len(), consumed), (1, 2));
    let (entries, consumed) = fill_page(&c, &[key("vanished")], 1000).unwrap();
    assert_eq!((entries.len(), consumed), (0, 1), "the cursor can still move past it");
}
