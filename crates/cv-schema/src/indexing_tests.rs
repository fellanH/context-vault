use super::*;

#[test]
fn test_explicit_indexed_true() {
    assert!(should_index("task", "event", 100, Some(true), None));
}

#[test]
fn test_explicit_indexed_false() {
    assert!(!should_index("insight", "knowledge", 100, Some(false), None));
}

#[test]
fn test_default_config_allows_indexing() {
    assert!(should_index("insight", "knowledge", 100, None, None));
    assert!(should_index("task", "event", 100, None, None));
}

#[test]
fn test_exclude_kinds() {
    let config = IndexingConfig {
        exclude_kinds: vec!["task".to_string()],
        exclude_categories: vec![],
        max_body_size: 0,
        auto_index_events: true,
    };

    assert!(!should_index("task", "event", 100, None, Some(&config)));
    assert!(should_index("session", "event", 100, None, Some(&config)));
}

#[test]
fn test_exclude_categories() {
    let config = IndexingConfig {
        exclude_kinds: vec![],
        exclude_categories: vec!["event".to_string()],
        max_body_size: 0,
        auto_index_events: true,
    };

    assert!(!should_index("task", "event", 100, None, Some(&config)));
    assert!(should_index("insight", "knowledge", 100, None, Some(&config)));
}

#[test]
fn test_auto_index_events_false() {
    let config = IndexingConfig {
        exclude_kinds: vec![],
        exclude_categories: vec![],
        max_body_size: 0,
        auto_index_events: false,
    };

    assert!(!should_index("task", "event", 100, None, Some(&config)));
    assert!(should_index("insight", "knowledge", 100, None, Some(&config)));
}

#[test]
fn test_auto_index_events_true() {
    let config = IndexingConfig {
        exclude_kinds: vec![],
        exclude_categories: vec![],
        max_body_size: 0,
        auto_index_events: true,
    };

    assert!(should_index("task", "event", 100, None, Some(&config)));
}

#[test]
fn test_max_body_size_exceeds() {
    let config = IndexingConfig {
        exclude_kinds: vec![],
        exclude_categories: vec![],
        max_body_size: 1000,
        auto_index_events: true,
    };

    assert!(!should_index("insight", "knowledge", 2000, None, Some(&config)));
}

#[test]
fn test_max_body_size_within() {
    let config = IndexingConfig {
        exclude_kinds: vec![],
        exclude_categories: vec![],
        max_body_size: 1000,
        auto_index_events: true,
    };

    assert!(should_index("insight", "knowledge", 500, None, Some(&config)));
}

#[test]
fn test_max_body_size_zero_means_no_limit() {
    let config = IndexingConfig {
        exclude_kinds: vec![],
        exclude_categories: vec![],
        max_body_size: 0,
        auto_index_events: true,
    };

    assert!(should_index("insight", "knowledge", 100000, None, Some(&config)));
}
