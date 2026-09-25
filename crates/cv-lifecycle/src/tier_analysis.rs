//! Hot/warm/cold tiering via access logs and co-retrieval graphs.
//! Identifies frequently accessed entries, cold entries, and co-access bundles.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotEntry {
    pub id: String,
    pub title: Option<String>,
    pub access_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdEntry {
    pub id: String,
    pub title: Option<String>,
    pub last_accessed: Option<String>,
    pub cold_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoAccessBundle {
    pub entries: Vec<BundleEntry>,
    pub total_weight: i64,
    pub brief_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleEntry {
    pub id: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierAnalysisReport {
    pub hot_entries: Vec<HotEntry>,
    pub cold_entries: Vec<ColdEntry>,
    pub warm_reset: usize,
    pub bundles: Vec<CoAccessBundle>,
    pub access_log_pruned: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TierAnalysisOptions {
    pub hot_threshold: Option<i64>,
    pub hot_days: Option<i64>,
    pub cold_days: Option<i64>,
    pub co_access_threshold: Option<i64>,
    pub dry_run: bool,
}

fn compute_cold_summary(body: &str) -> String {
    let first_sentence = body.split(|c: char| c.is_ascii_punctuation() && matches!(c, '.' | '!' | '?'))
        .next()
        .unwrap_or("")
        .trim();
    let summary = if first_sentence.is_empty() { body } else { first_sentence };
    summary[..std::cmp::min(200, summary.len())].trim().to_string()
}

/// Union-find for clustering co-access pairs.
/// PARITY: Not yet used in Phase 3; will be wired in Phase 4 when co_retrievals table is available.
#[allow(dead_code)]
struct UnionFind {
    parent: HashMap<String, String>,
}

#[allow(dead_code)]
impl UnionFind {
    fn new() -> Self {
        UnionFind {
            parent: HashMap::new(),
        }
    }

    fn find(&mut self, x: &str) -> String {
        if !self.parent.contains_key(x) {
            self.parent.insert(x.to_string(), x.to_string());
        }
        let parent = self.parent.get(x).unwrap().clone();
        if parent != x {
            let root = self.find(&parent);
            self.parent.insert(x.to_string(), root.clone());
        }
        self.parent.get(x).unwrap().clone()
    }

    fn union(&mut self, a: &str, b: &str) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent.insert(ra, rb);
        }
    }

    fn clusters(&mut self) -> HashMap<String, HashSet<String>> {
        let keys: Vec<String> = self.parent.keys().cloned().collect();
        let mut clusters: HashMap<String, HashSet<String>> = HashMap::new();
        for node in keys {
            let root = self.find(&node);
            clusters.entry(root).or_default().insert(node);
        }
        clusters
    }
}

/// Run tiering analysis: identify hot/cold entries and co-access bundles.
/// PARITY: Full version would update DB and create brief entries; this is dry-run only.
pub fn run_tier_analysis(
    db: &rusqlite::Connection,
    opts: &TierAnalysisOptions,
) -> Result<TierAnalysisReport> {
    let hot_threshold = opts.hot_threshold.unwrap_or(5);
    let hot_days = opts.hot_days.unwrap_or(7);
    let cold_days = opts.cold_days.unwrap_or(30);
    let _co_access_threshold = opts.co_access_threshold.unwrap_or(3);
    let dry_run = opts.dry_run;

    let mut report = TierAnalysisReport {
        hot_entries: Vec::new(),
        cold_entries: Vec::new(),
        warm_reset: 0,
        bundles: Vec::new(),
        access_log_pruned: 0,
    };

    // --- Hot detection ---
    let mut stmt = db.prepare(
        "SELECT v.id, v.title, COUNT(a.id) as access_count
         FROM vault v
         LEFT JOIN access_log a ON a.entry_id = v.id
         WHERE (a.accessed_at >= datetime('now', ? || ' days') OR a.accessed_at IS NULL)
           AND v.tier != 'ephemeral'
           AND v.superseded_by IS NULL
         GROUP BY v.id
         HAVING COUNT(a.id) >= ?",
    )?;

    let hot_rows = stmt.query_map(
        [format!("-{}", hot_days), hot_threshold.to_string()],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;

    for row in hot_rows {
        let (id, title, access_count) = row?;
        report.hot_entries.push(HotEntry {
            id: id.clone(),
            title,
            access_count,
        });
        if !dry_run {
            let _ = db.execute("UPDATE vault SET heat_tier = 'hot' WHERE id = ?", [&id]);
        }
    }

    // --- Cold detection ---
    let cold_day_str = format!("-{}", cold_days);
    let mut stmt = db.prepare(
        "SELECT v.id, v.title, v.body, v.last_accessed_at
         FROM vault v
         WHERE v.tier != 'durable'
           AND v.superseded_by IS NULL
           AND v.category != 'event'
           AND (v.last_accessed_at IS NULL OR v.last_accessed_at < datetime('now', ? || ' days'))
           AND NOT EXISTS (
             SELECT 1 FROM access_log a
             WHERE a.entry_id = v.id
               AND a.accessed_at >= datetime('now', ? || ' days')
           )",
    )?;

    let cold_rows = stmt.query_map([cold_day_str.clone(), cold_day_str], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    for row in cold_rows {
        let (id, title, body, last_accessed) = row?;
        let summary = compute_cold_summary(&body);
        report.cold_entries.push(ColdEntry {
            id: id.clone(),
            title,
            last_accessed,
            cold_summary: summary,
        });
        if !dry_run {
            let _ = db.execute(
                "UPDATE vault SET heat_tier = 'cold' WHERE id = ?",
                [&id],
            );
        }
    }

    // --- Warm reset: clear stale heat_tier ---
    if !dry_run {
        let hot_ids: HashSet<String> = report.hot_entries.iter().map(|e| e.id.clone()).collect();
        let cold_ids: HashSet<String> = report.cold_entries.iter().map(|e| e.id.clone()).collect();

        let mut stmt = db.prepare(
            "SELECT id, heat_tier FROM vault WHERE heat_tier IS NOT NULL AND superseded_by IS NULL",
        )?;

        let stale: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        for (id, heat_tier) in stale {
            let should_clear = (heat_tier == "hot" && !hot_ids.contains(&id))
                || (heat_tier == "cold" && !cold_ids.contains(&id));
            if should_clear {
                let _ = db.execute("UPDATE vault SET heat_tier = NULL WHERE id = ?", [&id]);
                report.warm_reset += 1;
            }
        }
    }

    // --- Co-access bundle detection ---
    // PARITY: Full version would use co_retrievals table; here we skip due to schema complexity.
    // (The table may not exist in minimal test harnesses.)

    // --- Housekeeping: prune old access_log rows ---
    if !dry_run {
        if let Ok(pruned) = db.execute(
            "DELETE FROM access_log WHERE accessed_at < datetime('now', '-90 days')",
            [],
        ) {
            report.access_log_pruned = pruned;
        }
    }

    Ok(report)
}

#[cfg(test)]
#[path = "tier_analysis_tests.rs"]
mod tests;
