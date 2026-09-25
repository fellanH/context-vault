//! Deduplication via cosine similarity, merging, and decay scoring.
//! Finds duplicate entries and merges them, computing decay scores for aging entries.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub canonical_id: String,
    pub duplicate_ids: Vec<String>,
    pub similarity: f32,
    pub importance_signal: usize,
    pub canonical_title: String,
    pub sample_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub canonical_id: String,
    pub merged_count: usize,
    pub new_recall_count: i64,
    pub removed_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayScore {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub score: usize,
    pub days_since_recall: i64,
    pub recall_count: i64,
    pub recommendation: String,
}

static KIND_WEIGHTS: &[(&str, f32)] = &[
    ("insight", 10.0),
    ("decision", 8.0),
    ("pattern", 7.0),
    ("reference", 5.0),
    ("event", 2.0),
];

fn kind_weight(kind: &str) -> f32 {
    KIND_WEIGHTS
        .iter()
        .find(|(k, _)| k == &kind)
        .map(|(_, w)| w)
        .copied()
        .unwrap_or(5.0)
}

/// Stub for finding duplicates via cosine similarity.
/// PARITY: Full version requires embeddings from vault_vec table;
/// this is a read-only dry-run that returns an empty list.
pub fn find_duplicates(
    _db: &rusqlite::Connection,
    _opts: &FindDuplicatesOptions,
) -> anyhow::Result<Vec<DuplicateGroup>> {
    Ok(Vec::new())
}

#[derive(Debug, Clone, Default)]
pub struct FindDuplicatesOptions {
    pub threshold: Option<f32>,
    pub limit: Option<usize>,
    pub kind: Option<String>,
    pub dry_run: bool,
}

/// Merge duplicate entries: sum recall counts, mark duplicates as superseded.
pub fn merge_entries(
    db: &rusqlite::Connection,
    canonical_id: &str,
    duplicate_ids: &[String],
    dry_run: bool,
) -> anyhow::Result<MergeResult> {
    if duplicate_ids.is_empty() {
        let canonical = db
            .query_row(
                "SELECT recall_count FROM vault WHERE id = ?",
                [canonical_id],
                |row| row.get::<_, i64>(0),
            )
            .ok();
        return Ok(MergeResult {
            canonical_id: canonical_id.to_string(),
            merged_count: 0,
            new_recall_count: canonical.unwrap_or(0),
            removed_ids: Vec::new(),
        });
    }

    // Gather recall stats
    let all_ids: Vec<String> = std::iter::once(canonical_id.to_string())
        .chain(duplicate_ids.iter().cloned())
        .collect();

    let mut total_recall_count = 0i64;
    let mut total_recall_sessions = 0i64;

    for id in &all_ids {
        if let Ok((count, sessions)) = db.query_row(
            "SELECT recall_count, recall_sessions FROM vault WHERE id = ?",
            [id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        ) {
            total_recall_count += count;
            total_recall_sessions += sessions;
        }
    }

    if dry_run {
        return Ok(MergeResult {
            canonical_id: canonical_id.to_string(),
            merged_count: duplicate_ids.len(),
            new_recall_count: total_recall_count,
            removed_ids: duplicate_ids.to_vec(),
        });
    }

    let now = chrono::Utc::now().to_rfc3339();
    let date_str = &now[..10];

    // Update canonical
    let consolidation_note =
        format!("\n\n[Consolidated from {} entries on {}]", duplicate_ids.len() + 1, date_str);

    db.execute(
        "UPDATE vault SET recall_count = ?, recall_sessions = ?, updated_at = ?,
         body = body || ? WHERE id = ?",
        [
            &total_recall_count.to_string(),
            &total_recall_sessions.to_string(),
            &now,
            &consolidation_note,
            canonical_id,
        ],
    )?;

    // Mark duplicates as superseded
    for dup_id in duplicate_ids {
        db.execute(
            "UPDATE vault SET superseded_by = ?, updated_at = ? WHERE id = ?",
            [canonical_id, &now, dup_id],
        )?;
    }

    Ok(MergeResult {
        canonical_id: canonical_id.to_string(),
        merged_count: duplicate_ids.len(),
        new_recall_count: total_recall_count,
        removed_ids: duplicate_ids.to_vec(),
    })
}

/// Compute decay scores for entries, recommending keep/compact/archive.
pub fn compute_decay_scores(
    db: &rusqlite::Connection,
    limit: Option<usize>,
    min_age_days: Option<i64>,
) -> anyhow::Result<Vec<DecayScore>> {
    let limit = limit.unwrap_or(100);
    let min_age_days = min_age_days.unwrap_or(0);
    let now = chrono::Utc::now().timestamp_millis();

    let mut where_parts = vec![
        "indexed = 1",
        "superseded_by IS NULL",
        "(expires_at IS NULL OR expires_at > datetime('now'))",
    ];
    let mut params: Vec<String> = Vec::new();

    if min_age_days > 0 {
        where_parts.push("created_at <= ?");
        let cutoff = chrono::DateTime::from_timestamp_millis(now - min_age_days * 86_400_000)
            .unwrap()
            .to_rfc3339();
        params.push(cutoff);
    }

    let query = format!(
        "SELECT id, title, kind, recall_count, last_recalled_at, updated_at, created_at
         FROM vault WHERE {} ORDER BY updated_at ASC",
        where_parts.join(" AND ")
    );

    let mut stmt = db.prepare(&query)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;

    let mut scores: Vec<DecayScore> = Vec::new();

    for row in rows {
        let (id, title, kind, recall_count, last_recalled_at, updated_at, created_at) = row?;

        let last_activity = last_recalled_at
            .or(updated_at)
            .unwrap_or(created_at);
        let last_activity_ms = chrono::DateTime::parse_from_rfc3339(&last_activity)
            .ok()
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(now);
        let days_since_recall = (now - last_activity_ms) / 86_400_000;

        let base_value = kind_weight(&kind);
        let time_decay = (-0.02 * days_since_recall as f32).exp();
        let recall_boost = 1.0 + ((recall_count as f32 + 1.0).ln());

        let raw_score = base_value * time_decay * recall_boost;
        let score = (raw_score * 10.0).clamp(0.0, 100.0) as usize;

        let recommendation = if score >= 30 {
            "keep".to_string()
        } else if score >= 10 {
            "compact".to_string()
        } else {
            "archive".to_string()
        };

        scores.push(DecayScore {
            id,
            title: title.unwrap_or_else(|| "(untitled)".to_string()),
            kind,
            score,
            days_since_recall,
            recall_count,
            recommendation,
        });
    }

    scores.sort_by_key(|s| s.score);
    Ok(scores.into_iter().take(limit).collect())
}

#[cfg(test)]
#[path = "consolidation_tests.rs"]
mod tests;
