use super::*;

#[test]
fn test_parse_frontmatter_basic() {
    let text = "---\nid: test123\nkind: insight\n---\nBody content here";
    let (meta, body) = parse_frontmatter(text);
    assert_eq!(meta.get("id").and_then(|v| v.as_str()), Some("test123"));
    assert_eq!(meta.get("kind").and_then(|v| v.as_str()), Some("insight"));
    assert_eq!(body, "Body content here");
}

#[test]
fn test_parse_frontmatter_multiline_tags() {
    let text = "---\ntags:\n  - tag1\n  - tag2\n---\nBody";
    let (meta, _body) = parse_frontmatter(text);
    if let Some(Value::Array(tags)) = meta.get("tags") {
        assert_eq!(tags.len(), 2);
    } else {
        panic!("tags should be an array");
    }
}

#[test]
fn test_parse_frontmatter_arrays() {
    let text = "---\nsupersedes: [\"id1\", \"id2\"]\n---\nBody";
    let (meta, _body) = parse_frontmatter(text);
    if let Some(Value::Array(supersedes)) = meta.get("supersedes") {
        assert_eq!(supersedes.len(), 2);
    } else {
        panic!("supersedes should be an array");
    }
}

#[test]
fn test_extract_custom_meta() {
    let mut map = Map::new();
    map.insert("id".to_string(), Value::String("test".to_string()));
    map.insert("custom_field".to_string(), Value::String("custom_value".to_string()));
    let custom = extract_custom_meta(&map);
    assert!(custom.is_some());
    let custom = custom.unwrap();
    assert!(!custom.contains_key("id"));
    assert!(custom.contains_key("custom_field"));
}

#[test]
fn typed_metadata_round_trips_without_coercing_quoted_strings_or_ids() {
    let fields = vec![
        ("id".into(), serde_json::json!("123")),
        ("amount".into(), serde_json::json!(9)),
        ("ratio".into(), serde_json::json!(0.25)),
        ("enabled".into(), serde_json::json!(true)),
        ("numeric_text".into(), serde_json::json!("9")),
        ("boolean_text".into(), serde_json::json!("true")),
        ("null_text".into(), serde_json::json!("null")),
        ("details".into(), serde_json::json!({"count":2,"safe":true})),
    ];
    let raw = crate::frontmatter::format_frontmatter(&fields) + "\nBody";
    let (parsed, _) = parse_frontmatter(&raw);
    for (key, value) in fields { assert_eq!(parsed.get(&key), Some(&value), "{key}"); }
}
