use super::*;
use serde_json::json;

const MARKERS: &[&str] = &["★ Insight"];

fn assistant_turn(text: &str) -> Value {
    json!({"role": "assistant", "content": text})
}

#[test]
fn extracts_a_marked_insight_block() {
    let text = "★ Insight\n──────────\nThis is the insight body which needs to be at least \
                fifty characters total to pass the filter.\n──────────";
    let turns = vec![assistant_turn(text)];

    let found = extract_insights(&turns, MARKERS, 50, 5);

    assert_eq!(found.len(), 1);
    assert!(found[0].body.starts_with("This is the insight body"));
}

#[test]
fn ignores_non_assistant_turns() {
    let text = "★ Insight\n──────────\nThis is the insight body which needs to be at least \
                fifty characters total to pass the filter.\n──────────";
    let turns = vec![json!({"role": "user", "content": text})];

    assert!(extract_insights(&turns, MARKERS, 50, 5).is_empty());
}

#[test]
fn filters_blocks_shorter_than_min_chars() {
    let text = "★ Insight\n──────────\ntoo short\n──────────";
    let turns = vec![assistant_turn(text)];

    assert!(extract_insights(&turns, MARKERS, 50, 5).is_empty());
}

#[test]
fn caps_at_max_per_session() {
    let one_block = "★ Insight\n──────────\nThis is a sufficiently long insight body for the \
                      test to pass the character threshold check.\n──────────\n";
    let text = one_block.repeat(3);
    let turns = vec![assistant_turn(&text)];

    let found = extract_insights(&turns, MARKERS, 50, 2);
    assert_eq!(found.len(), 2);
}

#[test]
fn title_prefers_bold_text_over_first_line() {
    let body = "**Express 5 gotcha**: the body parser changed in a breaking way.";
    assert_eq!(insight_title(body), "Express 5 gotcha");
}

#[test]
fn title_falls_back_to_first_line() {
    let body = "no bold text here, just a plain first line\nsecond line";
    assert_eq!(insight_title(body), "no bold text here, just a plain first line");
}
