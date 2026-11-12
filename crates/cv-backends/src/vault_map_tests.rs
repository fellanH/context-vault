use chrono::{Duration, Utc};
use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::vault_map::{get, is_stale};
use crate::vault_map_areas::summarize;

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch(include_str!("../../../fixtures/bench/reference_schema.sql")).unwrap();
    c
}

fn seeded() -> Connection {
    let c = conn();
    for i in 0..12 {
        let meta = (i == 0).then_some(r#"{"situations":["fresh"]}"#);
        c.execute(
            "INSERT INTO vault (id, kind, category, title, body, tags, meta, created_at, updated_at) VALUES (?1,'insight','knowledge',?2,?3,?4,?5,?6,?6)",
            params![format!("i{i:02}"), format!("Insight {i}"), format!("Body {i}. More."), r#"["rust","sqlite"]"#, meta, format!("2026-01-{:02}T00:00:00Z", i + 1)],
        )
        .unwrap();
    }
    c.execute("INSERT INTO vault (id, kind, category, title, body, created_at, updated_at, expires_at) VALUES ('old','insight','knowledge','Old','x','2026-01-01','2026-01-01','2001-01-01T00:00:00Z')", []).unwrap();
    c.execute("INSERT INTO vault (id, kind, category, body, created_at, updated_at) VALUES ('d1','decision','knowledge','Decided to ship.','2026-02-01','2026-02-01')", []).unwrap();
    c
}

#[test]
fn summary_takes_the_first_sentence_without_markdown() {
    assert_eq!(summarize("# Title\n\n**Bold** start. Second sentence."), "Title Bold start.");
    assert_eq!(summarize("```\ncode\n```\nPlain text here"), "Plain text here");
    assert_eq!(summarize(""), "");
    assert!(summarize(&"word ".repeat(100)).chars().count() <= 140);
}

#[test]
fn totals_and_areas() {
    let map = get(&seeded(), false, Utc::now()).unwrap();
    assert_eq!(map["regenerated"], true);
    assert_eq!(map["totals"]["live"], 13, "the expired entry is not counted");
    assert_eq!(map["areas"][0]["category"], "knowledge");
    assert_eq!(map["areas"][0]["entries"], 13);
    let kinds = map["areas"][0]["kinds"].as_array().unwrap();
    assert_eq!((kinds[0]["kind"].as_str(), kinds[0]["entries"].as_i64()), (Some("insight"), Some(12)));
    assert_eq!(kinds[1]["key_entries"][0]["title"], "Decided to ship.");
}

#[test]
fn key_entries_rank_situated_first() {
    let map = get(&seeded(), false, Utc::now()).unwrap();
    let keys = map["areas"][0]["kinds"][0]["key_entries"].as_array().unwrap().clone();
    assert_eq!(keys.len(), 8);
    assert_eq!(keys[0]["id"], "i00");
    assert_eq!(keys[0]["situations"], json!(["fresh"]));
    assert_eq!(keys[0]["summary"], "Body 0.");
    assert_eq!(map["tags"][0], json!({"tag": "rust", "entries": 12}));
    assert_eq!(map["situations"], json!([{"situation": "fresh", "entries": 1}]));
}

#[test]
fn cache_and_refresh_window() {
    let c = seeded();
    let now = Utc::now();
    get(&c, false, now).unwrap();
    let again = |force: bool, at| get(&c, force, at).unwrap()["regenerated"].clone();
    assert_eq!(again(false, now), Value::Bool(false));
    assert_eq!(again(true, now), Value::Bool(false), "a forced refresh within a minute returns the cache");
    assert_eq!(again(true, now + Duration::seconds(61)), Value::Bool(true));
    assert_eq!(again(false, now + Duration::hours(25)), Value::Bool(true));
}

#[test]
fn staleness_rules() {
    let now = Utc::now();
    let at = now.to_rfc3339();
    assert!(!is_stale(&at, 1000, 1010, now));
    assert!(is_stale(&at, 1000, 1021, now), "more than 2%");
    assert!(is_stale(&at, 100_000, 100_051, now), "more than 50");
    assert!(!is_stale(&at, 100_000, 100_049, now));
    assert!(is_stale(&at, 10, 12, now), "2 of 10 is over 2%");
    assert!(is_stale("garbage", 10, 10, now));
}
