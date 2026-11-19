use super::{Entry, ScoredEntry};
use chrono::DateTime;
use std::collections::HashMap;

const RRF_K: f32 = 60.0;

/// Reciprocal-rank fusion of the full-text and semantic lanes, then tier, heat and recency boosts.
///
/// Match strength comes first (OPS-468): the first `every_word` full-text hits held every word of
/// the query, and they print before any hit that held only some words or matched by meaning alone,
/// however old they are. The boosts order hits inside one strength block only, because a boost
/// (tier 0.7 to 1.5, heat 0.7 to 1.3, recency down to 0.1) outweighs many steps of rank (about
/// 1.6 % apart), so a 60-day-old exact match used to print below a fresh one-word hit.
pub fn rrf_fuse(
    fts_results: Vec<(Entry, f32)>,
    every_word: usize,
    vec_results: Vec<(Entry, f32)>,
    tier_boosts: &HashMap<String, f32>,
    heat_boosts: &HashMap<String, f32>,
) -> Vec<ScoredEntry> {
    let mut scores: HashMap<String, ScoreComponents> = HashMap::new();
    for (idx, (entry, rank)) in fts_results.iter().enumerate() {
        let c = scores.entry(entry.id.clone()).or_insert_with(|| ScoreComponents::new(entry));
        c.fts_score = Some(1.0 / (RRF_K + idx as f32));
        c.fts_rank = Some(*rank);
        c.block = if idx < every_word { 0 } else { 1 };
    }
    for (idx, (entry, rank)) in vec_results.iter().enumerate() {
        let c = scores.entry(entry.id.clone()).or_insert_with(|| ScoreComponents::new(entry));
        c.vec_score = Some(1.0 / (RRF_K + idx as f32));
        c.vec_rank = Some(*rank);
    }
    let mut results: Vec<(u8, ScoredEntry)> = scores.into_values().map(|c| (c.block, c.scored(tier_boosts, heat_boosts))).collect();
    results.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.score.partial_cmp(&a.1.score).unwrap_or(std::cmp::Ordering::Equal)));
    results.into_iter().map(|(_, s)| s).collect()
}

fn recency_decay(updated_at: &str) -> f32 {
    if let Ok(dt) = DateTime::parse_from_rfc3339(updated_at) {
        let now = chrono::Utc::now();
        let age_secs = (now - dt.with_timezone(&chrono::Utc)).num_seconds().max(0) as f32;
        let age_days = age_secs / 86400.0;
        (1.0 / (1.0 + age_days / 30.0)).max(0.1)
    } else {
        0.5
    }
}

struct ScoreComponents {
    entry: Entry,
    fts_score: Option<f32>,
    vec_score: Option<f32>,
    fts_rank: Option<f32>,
    vec_rank: Option<f32>,
    /// 0: held every query word; 1: some words, or meaning only.
    block: u8,
}

impl ScoreComponents {
    fn new(entry: &Entry) -> Self {
        Self { entry: entry.clone(), fts_score: None, vec_score: None, fts_rank: None, vec_rank: None, block: 1 }
    }

    /// The fused score with tier, heat and recency applied.
    fn scored(self, tier_boosts: &HashMap<String, f32>, heat_boosts: &HashMap<String, f32>) -> ScoredEntry {
        let fused = (self.fts_score.unwrap_or(0.0) + self.vec_score.unwrap_or(0.0)) * 100.0;
        let tier = tier_boosts.get(&self.entry.tier).copied().unwrap_or(1.0);
        let heat = self.entry.heat_tier.as_ref().and_then(|h| heat_boosts.get(h)).copied().unwrap_or(1.0);
        let score = fused * tier * heat * recency_decay(&self.entry.updated_at);
        ScoredEntry { entry: self.entry, score, fts_rank: self.fts_rank, vec_rank: self.vec_rank }
    }
}
