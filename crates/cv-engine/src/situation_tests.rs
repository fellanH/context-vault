use serde_json::{json, Value};

use crate::situation_meta::apply_to_meta;
use crate::situations::{normalize_situations, valid_name};

#[test]
fn names() {
    assert!(valid_name("post-compaction") && valid_name("a1") && valid_name("fresh"));
    for bad in ["", "Fresh", "a--b", "-a", "a-", "a b", "a_b", &"x".repeat(33)] {
        assert!(!valid_name(bad), "{bad}");
    }
}

#[test]
fn normalize_dedupes_and_caps() {
    assert_eq!(normalize_situations(&json!(["a", "b", "a"])).unwrap(), vec!["a", "b"]);
    assert!(normalize_situations(&json!(["a", "b", "c", "d", "e", "f", "g"])).is_err());
    assert!(normalize_situations(&json!("a")).is_err());
    assert!(normalize_situations(&json!([1])).is_err());
}

#[test]
fn apply_sets_and_reorders() {
    let m = apply_to_meta(None, None, Some(&json!(["fresh"])), Some(&json!(5))).unwrap().unwrap();
    assert_eq!(m, json!({"situations": ["fresh"], "situation_order": 5}));
    let m2 = apply_to_meta(Some(m), None, None, Some(&json!(9))).unwrap().unwrap();
    assert_eq!(m2["situation_order"], 9);
}

#[test]
fn apply_carries_and_clears() {
    let stored = json!({"situations": ["fresh"], "situation_order": 5});
    let m = apply_to_meta(Some(json!({"x": 1})), Some(&stored), None, None).unwrap().unwrap();
    assert_eq!(m, json!({"x": 1, "situations": ["fresh"], "situation_order": 5}));
    // null clears, and takes the order with it
    assert_eq!(apply_to_meta(Some(stored), None, Some(&Value::Null), None).unwrap(), None);
}

#[test]
fn apply_rejects_bad_input() {
    assert!(apply_to_meta(None, None, None, Some(&json!(-1))).is_err());
    assert!(apply_to_meta(None, None, Some(&json!(["Bad"])), None).is_err());
    assert!(apply_to_meta(Some(json!({"situations": ["Bad"]})), None, None, None).is_err());
}

#[test]
fn stored_malformed_meta_stays_editable() {
    for stored in [
        json!({"situations": "fresh", "x": 1}),
        json!({"situations": ["Before Deploy"], "situation_order": 1.5}),
        json!({"situations": ["a", "b", "c", "d", "e", "f", "g"]}),
        json!("not an object"),
    ] {
        // nothing situation-related supplied: stored meta is left exactly as it is
        assert_eq!(apply_to_meta(None, Some(&stored), None, None).unwrap(), Some(stored.clone()));
        // a whole new meta carries the stored situation keys over untouched
        if stored.is_object() {
            let m = apply_to_meta(Some(json!({"y": 2})), Some(&stored), None, None).unwrap().unwrap();
            assert_eq!(m["situations"], stored["situations"]);
        }
    }
}
