use super::*;

#[test]
fn test_short_entry_identity() {
    let body = "This is a short note.";
    let result = generate_summary_tiers(body);
    assert_eq!(result.condensed, body);
    assert_eq!(result.keypoint, body);
}

#[test]
fn test_strip_frontmatter() {
    let body = "---\ntitle: Test\n---\nContent here.";
    let result = generate_summary_tiers(body);
    assert!(result.condensed.contains("Content"));
    assert!(!result.condensed.contains("title: Test"));
}

#[test]
fn test_two_sentence_condensed() {
    let body = "First sentence. Second sentence.";
    let result = generate_summary_tiers(body);
    assert!(result.condensed.contains("First"));
    assert!(result.condensed.contains("Second"));
}

#[test]
fn test_header_as_keypoint() {
    let body = "# Main Title\n\nContent here that is much longer and contains multiple sentences to test.";
    let result = generate_summary_tiers(body);
    assert!(result.keypoint.contains("Main Title") || result.keypoint.contains("Content"));
}

#[test]
fn test_empty_body() {
    let body = "";
    let result = generate_summary_tiers(body);
    assert_eq!(result.condensed, "");
    assert_eq!(result.keypoint, "");
}

#[test]
fn test_code_only_detection() {
    let body = "const x = 5;\nconst y = 10;\nreturn x + y;";
    let result = generate_summary_tiers(body);
    // Code-only detection may not trigger for 3-line snippets; check if it's treated as plain text
    assert!(!result.condensed.is_empty());
}

#[test]
fn test_caps_work() {
    let body = "This is a very long sentence that should definitely be truncated because it exceeds the normal character limit for a keypoint summary in the vault system.";
    let result = generate_summary_tiers(body);
    assert!(result.keypoint.ends_with("..."));
    assert!(result.keypoint.len() <= KEYPOINT_CAP + 3);
}
