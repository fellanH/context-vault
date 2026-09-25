use super::*;

#[test]
fn test_kind_weight() {
    assert_eq!(kind_weight("insight"), 10.0);
    assert_eq!(kind_weight("decision"), 8.0);
    assert_eq!(kind_weight("event"), 2.0);
    assert_eq!(kind_weight("unknown"), 5.0); // default
}

#[test]
fn test_merge_result_empty_duplicates() {
    // Verify that empty duplicate_ids returns 0 merged_count
    let result = MergeResult {
        canonical_id: "test".to_string(),
        merged_count: 0,
        new_recall_count: 5,
        removed_ids: Vec::new(),
    };
    assert_eq!(result.merged_count, 0);
}

#[test]
fn test_decay_score_recommendation_keep() {
    let score = DecayScore {
        id: "test".to_string(),
        title: "Test".to_string(),
        kind: "insight".to_string(),
        score: 50,
        days_since_recall: 5,
        recall_count: 10,
        recommendation: "keep".to_string(),
    };
    assert_eq!(score.recommendation, "keep");
}

#[test]
fn test_decay_score_recommendation_archive() {
    let score = DecayScore {
        id: "test".to_string(),
        title: "Test".to_string(),
        kind: "event".to_string(),
        score: 5,
        days_since_recall: 100,
        recall_count: 0,
        recommendation: "archive".to_string(),
    };
    assert_eq!(score.recommendation, "archive");
}
