use super::{Entry, ScoredEntry};
use chrono::DateTime;
use std::collections::HashMap;

const RRF_K: f32 = 60.0;

pub fn rrf_fuse(
    fts_results: Vec<(Entry, f32)>,
    vec_results: Vec<(Entry, f32)>,
    tier_boosts: &HashMap<String, f32>,
    heat_boosts: &HashMap<String, f32>,
) -> Vec<ScoredEntry> {
    let mut scores: HashMap<String, ScoreComponents> = HashMap::new();

    for (idx, (entry, rank)) in fts_results.iter().enumerate() {
        let fts_score = 1.0 / (RRF_K + (idx as f32));
        scores
            .entry(entry.id.clone())
            .or_insert_with(|| ScoreComponents {
                entry: entry.clone(),
                fts_score: Some(fts_score),
                vec_score: None,
                fts_rank: Some(*rank),
                vec_rank: None,
            });
    }

    for (idx, (entry, rank)) in vec_results.iter().enumerate() {
        let vec_score = 1.0 / (RRF_K + (idx as f32));
        scores
            .entry(entry.id.clone())
            .or_insert_with(|| ScoreComponents {
                entry: entry.clone(),
                fts_score: None,
                vec_score: Some(vec_score),
                fts_rank: None,
                vec_rank: Some(*rank),
            })
            .vec_score = Some(vec_score);
        scores.get_mut(&entry.id).unwrap().vec_rank = Some(*rank);
    }

    let mut results: Vec<ScoredEntry> = scores
        .into_values()
        .map(|components| {
            let fused = (components.fts_score.unwrap_or(0.0)
                + components.vec_score.unwrap_or(0.0))
                * 100.0;

            let tier_boost = tier_boosts
                .get(&components.entry.tier)
                .copied()
                .unwrap_or(1.0);
            let heat_boost = components
                .entry
                .heat_tier
                .as_ref()
                .and_then(|h| heat_boosts.get(h))
                .copied()
                .unwrap_or(1.0);

            let recency = recency_decay(&components.entry.updated_at);

            let final_score = fused * tier_boost * heat_boost * recency;

            ScoredEntry {
                entry: components.entry,
                score: final_score,
                fts_rank: components.fts_rank,
                vec_rank: components.vec_rank,
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
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
}
