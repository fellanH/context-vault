use super::*;
use serde_json::json;

#[test]
fn parses_hook_input() {
    let raw = r#"{"session_id":"abc123","transcript_path":"/tmp/t.jsonl","cwd":"/Users/x/my-project"}"#;
    let input = parse_hook_input(raw).unwrap();
    assert_eq!(input.session_id.as_deref(), Some("abc123"));
    assert_eq!(input.transcript_path, "/tmp/t.jsonl");
    assert_eq!(input.cwd, "/Users/x/my-project");
}

#[test]
fn missing_required_fields_returns_none() {
    assert!(parse_hook_input(r#"{"session_id":"abc"}"#).is_none());
    assert!(parse_hook_input("not json").is_none());
}

#[test]
fn parse_turns_skips_blank_and_invalid_lines() {
    let raw = "{\"role\":\"user\",\"content\":\"hi\"}\n\n  \nnot json\n{\"role\":\"assistant\",\"content\":\"hello\"}\n";
    let turns = parse_turns(raw);
    assert_eq!(turns.len(), 2);
}

#[test]
fn extract_text_handles_string_and_block_array() {
    let string_turn = json!({"role": "user", "content": "plain text"});
    assert_eq!(extract_text(&string_turn), "plain text");

    let block_turn = json!({
        "role": "assistant",
        "content": [
            {"type": "text", "text": "first"},
            {"type": "tool_use", "name": "Bash"},
            {"type": "text", "text": "second"}
        ]
    });
    assert_eq!(extract_text(&block_turn), "first second");
}

fn sample_turns() -> Vec<Value> {
    vec![
        json!({
            "role": "user",
            "content": "please fix the bug",
            "timestamp": "2026-07-03T10:00:00Z",
        }),
        json!({
            "role": "assistant",
            "content": [
                {"type": "text", "text": "looking into it"},
                {"type": "tool_use", "name": "Bash", "input": {"command": "cargo test --workspace"}},
                {"type": "tool_use", "name": "Edit", "input": {"file_path": "/repo/src/lib.rs"}},
                {"type": "tool_use", "name": "Edit", "input": {"file_path": "/repo/src/lib.rs"}},
                {"type": "tool_use", "name": "Write", "input": {"path": "/repo/src/new.rs"}},
            ],
            "timestamp": "2026-07-03T10:02:00Z",
        }),
        json!({
            "role": "user",
            "content": "looks good, thanks",
            "timestamp": "2026-07-03T10:05:00Z",
        }),
    ]
}

#[test]
fn build_signals_extracts_all_fields() {
    let turns = sample_turns();
    let signals = build_signals(&turns, "/Users/x/my-project").unwrap();

    assert_eq!(signals.project, "my-project");
    assert_eq!(signals.message_count, 2);
    assert_eq!(signals.first_prompt, "please fix the bug");
    assert_eq!(signals.last_prompt, "looks good, thanks");
    assert_eq!(signals.duration.as_deref(), Some("5m 0s"));
    // Edit on the same file twice -> deduped to one entry.
    assert_eq!(signals.files_modified, vec!["/repo/src/lib.rs", "/repo/src/new.rs"]);
    assert_eq!(signals.commands_run, vec!["cargo test --workspace"]);
    assert_eq!(signals.tool_counts[0], ("Edit".to_string(), 2));
}

#[test]
fn build_signals_none_without_user_turns() {
    let turns = vec![json!({"role": "assistant", "content": "hi"})];
    assert!(build_signals(&turns, "/x").is_none());
}

#[test]
fn format_session_body_includes_all_sections() {
    let turns = sample_turns();
    let signals = build_signals(&turns, "/Users/x/my-project").unwrap();
    let body = format_session_body(&signals);

    assert!(body.contains("Session in my-project (2 exchanges, ~5m 0s)"));
    assert!(body.contains("Opened with: please fix the bug"));
    assert!(body.contains("Closed with: looks good, thanks"));
    assert!(body.contains("## Files modified"));
    assert!(body.contains("- /repo/src/lib.rs"));
    assert!(body.contains("## Key commands"));
    assert!(body.contains("- cargo test --workspace"));
    assert!(body.contains("## Tools used"));
    assert!(body.contains("Edit: 2"));
}

#[test]
fn capped_section_reports_overflow() {
    let files: Vec<String> = (0..25).map(|i| format!("file-{i}.rs")).collect();
    let mut lines = Vec::new();
    push_capped_section(&mut lines, "## Files modified", &files, 20);
    let joined = lines.join("\n");
    assert!(joined.contains("- ... and 5 more"));
}
