use super::*;

#[test]
fn test_role_as_str() {
    assert_eq!(Role::Worker.as_str(), "worker");
    assert_eq!(Role::PM.as_str(), "pm");
    assert_eq!(Role::CEO.as_str(), "ceo");
    assert_eq!(Role::Steer.as_str(), "steer");
}

#[test]
fn test_default_role_profiles() {
    let profiles = default_role_profiles();
    assert!(profiles.contains_key("worker"));
    assert!(profiles.contains_key("pm"));
    assert!(profiles.contains_key("ceo"));
    assert!(profiles.contains_key("steer"));
}

#[test]
fn test_assemble_worker_role() {
    let result = assemble_context(Role::Worker, "Implement feature X", 1000);
    assert!(result.markdown.contains("WORKER"));
    assert!(result.markdown.contains("Implement feature X"));
    assert!(result.markdown.contains("Role Directives"));
    assert_eq!(result.metadata.role, "worker");
    assert_eq!(result.metadata.budget, 1000);
}

#[test]
fn test_assemble_ceo_role() {
    let result = assemble_context(Role::CEO, "Strategic decision", 2000);
    assert!(result.markdown.contains("CEO"));
    assert!(result.markdown.contains("Strategic decision"));
    assert_eq!(result.metadata.role, "ceo");
}

#[test]
fn test_metadata_token_calculation() {
    let result = assemble_context(Role::Worker, "Test", 500);
    assert!(result.metadata.tokens_used > 0);
    assert!(result.metadata.tokens_used <= 500);
}
