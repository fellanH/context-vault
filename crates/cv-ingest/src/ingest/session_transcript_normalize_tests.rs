use super::*;

/// A real Claude Code transcript excerpt: `role`/`content` nested under `message`, mixed
/// in with the non-message line types a live transcript actually carries
/// (`last-prompt`, `mode`, `permission-mode`, `attachment`, `ai-title`, `system`). Shape
/// is faithful to production; content is neutral (no private paths) per the sellable line.
const REAL_TRANSCRIPT: &str = r#"{"type":"last-prompt","leafUuid":"x","sessionId":"s1"}
{"type":"mode","mode":"normal","sessionId":"s1"}
{"type":"permission-mode","permissionMode":"auto","sessionId":"s1"}
{"type":"attachment","sessionId":"s1"}
{"type":"user","message":{"role":"user","content":"please fix the parser bug"},"timestamp":"2026-07-03T10:00:00Z","cwd":"/w/demo-project"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"hmm"},{"type":"text","text":"★ Insight\n**Nested transcript shape** — real Claude Code lines nest role and content under message, so top-level reads capture nothing.\n────────────────────"},{"type":"tool_use","name":"Bash","input":{"command":"cargo test --workspace"}},{"type":"tool_use","name":"Edit","input":{"file_path":"/repo/src/lib.rs"}}]},"timestamp":"2026-07-03T10:02:00Z"}
{"type":"ai-title","title":"fix parser","sessionId":"s1"}
{"type":"system","sessionId":"s1"}
{"type":"user","message":{"role":"user","content":"looks good, thanks"},"timestamp":"2026-07-03T10:05:00Z"}"#;

#[test]
fn normalize_turns_handles_real_nested_transcript() {
    let turns = normalize_turns(parse_turns(REAL_TRANSCRIPT));
    // Only the 3 message lines survive; the 5 non-message noise types are dropped.
    assert_eq!(turns.len(), 3);

    let signals = build_signals(&turns, "/w/demo-project").unwrap();
    assert_eq!(signals.project, "demo-project");
    assert_eq!(signals.message_count, 2); // two user turns
    assert_eq!(signals.first_prompt, "please fix the parser bug");
    assert_eq!(signals.last_prompt, "looks good, thanks");
    assert_eq!(signals.duration.as_deref(), Some("5m 0s"));
    assert_eq!(signals.files_modified, vec!["/repo/src/lib.rs"]);
    assert_eq!(signals.commands_run, vec!["cargo test --workspace"]);
    assert!(signals.tool_counts.iter().any(|(n, _)| n == "Edit"));

    // The insight path reads the same normalized turns -- prove it fires on real shape.
    let insights = crate::ingest::extract_insights(&turns, &["★ Insight"], 50, 5);
    assert_eq!(insights.len(), 1);
    assert_eq!(insights[0].title, "Nested transcript shape");
}

#[test]
fn normalize_turns_drops_lines_without_message() {
    // A `user`-typed line missing its `message` object is skipped, not panicked on.
    let raw = parse_turns(
        "{\"type\":\"user\",\"sessionId\":\"s1\"}\n{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"real\"},\"timestamp\":\"2026-07-03T10:00:00Z\"}",
    );
    let turns = normalize_turns(raw);
    assert_eq!(turns.len(), 1);
}
