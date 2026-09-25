use super::*;

#[test]
fn test_multiple_constraints() {
    let config = IndexingConfig {
        exclude_kinds: vec!["task".to_string()],
        exclude_categories: vec![],
        max_body_size: 1000,
        auto_index_events: true,
    };

    assert!(should_index("insight", "knowledge", 500, None, Some(&config)));
    assert!(!should_index("task", "event", 500, None, Some(&config)));
    assert!(!should_index("insight", "knowledge", 2000, None, Some(&config)));
}

#[test]
fn test_explicit_indexed_overrides_config() {
    let config = IndexingConfig {
        exclude_kinds: vec!["task".to_string()],
        exclude_categories: vec![],
        max_body_size: 500,
        auto_index_events: false,
    };

    assert!(should_index("task", "event", 1000, Some(true), Some(&config)));
}

#[test]
fn test_explicit_false_overrides_config() {
    let config = IndexingConfig {
        exclude_kinds: vec![],
        exclude_categories: vec![],
        max_body_size: 0,
        auto_index_events: true,
    };

    assert!(!should_index("task", "event", 100, Some(false), Some(&config)));
}

#[test]
fn test_default_indexing_config() {
    let config = IndexingConfig::default();
    assert_eq!(config.exclude_kinds.len(), 0);
    assert_eq!(config.exclude_categories.len(), 0);
    assert_eq!(config.max_body_size, 0);
    assert!(config.auto_index_events);
}
