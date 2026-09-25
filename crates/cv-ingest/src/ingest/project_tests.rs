use super::*;

#[test]
fn test_slugify() {
    assert_eq!(slugify("My Project"), "my-project");
    assert_eq!(slugify("@scope/my-lib"), "scope-my-lib");
}

#[test]
fn test_detect_tech_stack() {
    let empty_stack = detect_tech_stack("/nonexistent", None);
    assert!(empty_stack.is_empty());
}
