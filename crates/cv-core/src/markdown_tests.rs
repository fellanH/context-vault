use crate::markdown::{read_record, write_record};
use crate::record::Record;

#[test]
fn markdown_round_trip_keeps_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let mut r = Record::new("insight", "Tantivy's top-k collector barely slows as hits grow.");
    r.title = Some("tantivy top-k".into());
    r.tags = vec!["project:context-vault".into(), "topic:search".into()];
    r.meta.insert("measured".into(), serde_json::json!("2026-09-25"));
    r.source = Some("benchmark".into());
    let (written, path) = write_record(tmp.path(), &r).unwrap();
    let back = read_record(&path).unwrap();
    assert_eq!(back, written);
    assert_eq!((back.kind.as_str(), back.title.as_deref(), back.source.as_deref()), ("insight", Some("tantivy top-k"), Some("benchmark")));
    assert_eq!(back.tags, r.tags);
    assert_eq!(back.meta.get("measured"), r.meta.get("measured"));
    assert!(back.text.contains("top-k collector"), "{}", back.text);
}

#[test]
fn markdown_writes_still_pass_the_refusers() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("vault.toml"), "schema_version = 1\n[vault]\npersonal_free = true\n").unwrap();
    let mut r = Record::new("gotcha", "x");
    r.meta.insert("sensitivity".into(), serde_json::json!("personal"));
    assert!(write_record(tmp.path(), &r).unwrap_err().to_string().contains("personal-free"));
}
