use super::*;

#[test]
fn test_compute_cold_summary_short() {
    let body = "Quick summary.";
    let summary = compute_cold_summary(body);
    assert_eq!(summary, "Quick summary");
}

#[test]
fn test_compute_cold_summary_long() {
    let body = "This is a long sentence. And another one. And another one.";
    let summary = compute_cold_summary(body);
    assert!(summary.len() <= 200);
    assert_eq!(summary, "This is a long sentence");
}

#[test]
fn test_union_find_basic() {
    let mut uf = UnionFind::new();
    uf.union("a", "b");
    uf.union("b", "c");
    let ra = uf.find("a");
    let rc = uf.find("c");
    assert_eq!(ra, rc); // Should be in same cluster
}

#[test]
fn test_union_find_clusters() {
    let mut uf = UnionFind::new();
    uf.union("a", "b");
    uf.union("b", "c");
    uf.union("x", "y");
    let clusters = uf.clusters();
    assert_eq!(clusters.len(), 2);
}

#[test]
fn test_tier_analysis_options_defaults() {
    let opts = TierAnalysisOptions::default();
    assert!(!opts.dry_run);
}
