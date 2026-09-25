//! Gzip cold entry bodies to _archive/ and replace with summaries.
//! Uses flate2 for compression; operates on markdown via vault files.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompactEntry {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub body_size: usize,
    pub heat: f32,
    pub tier: Option<String>,
    pub age_days: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CompactResult {
    pub candidates: usize,
    pub compacted: usize,
    pub bytes_reclaimed: usize,
    pub entries: Vec<CompactEntry>,
}

fn archive_gz_path(vault_dir: &str, id: &str) -> PathBuf {
    Path::new(vault_dir)
        .join("_archive")
        .join(format!("{}.md.gz", id))
}

fn ensure_archive_dir(vault_dir: &str) -> Result<()> {
    let dir = Path::new(vault_dir).join("_archive");
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

/// Summarize body for compacted entries: truncate and mark as archived.
pub fn summarize_body(body: &str) -> String {
    if body.len() <= 500 {
        body.to_string()
    } else {
        format!("{}...\n\n[compacted: full body archived]", &body[..500])
    }
}

/// Find entries eligible for compaction: heat_tier is NULL or 'cold',
/// age > 90 days (or 30 for explicit cold tier), recall_count = 0.
///
/// For now, this is a dry-run only version that doesn't modify the DB.
/// Pass `dry_run: true` to simulate; `false` would require DB handle.
pub fn find_compact_candidates(
    db: &rusqlite::Connection,
    options: &CompactOptions,
) -> Result<Vec<CompactEntry>> {
    let min_age_days = if options.tier.as_deref() == Some("cold") { 30 } else { 90 };
    let now = chrono::Utc::now().timestamp_millis();
    let cutoff_ms = now - (min_age_days as i64 * 86_400_000);
    let cutoff_iso = chrono::DateTime::from_timestamp_millis(cutoff_ms)
        .ok_or_else(|| anyhow!("invalid timestamp"))?
        .to_rfc3339();

    let mut stmt = db.prepare(
        "SELECT id, kind, title, body, created_at, heat_tier
         FROM vault
         WHERE recall_count = 0
           AND created_at <= ?
           AND indexed != 0
         ORDER BY created_at ASC",
    )?;

    let candidates: Vec<CompactEntry> = stmt
        .query_map([cutoff_iso], |row| {
            let id: String = row.get(0)?;
            let kind: String = row.get(1)?;
            let title: Option<String> = row.get(2)?;
            let body: String = row.get(3)?;
            let created_at: String = row.get(4)?;
            let heat_tier: Option<String> = row.get(5)?;

            let body_size = body.len();
            let age_ms = now
                - created_at
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now())
                    .timestamp_millis();
            let age_days = age_ms / 86_400_000;

            // Compute heat: placeholder (0.0 for now; full version would use
            // computeHeatForEntry from search module)
            let heat = 0.0;

            // Eligible: tier is NULL or 'cold' in default mode; with --tier cold also include 'cold'
            let eligible = match options.tier.as_deref() {
                Some("cold") => heat_tier.as_deref() == Some("cold") || heat_tier.is_none(),
                _ => heat_tier.is_none() || heat_tier.as_deref() == Some("cold"),
            };

            Ok((
                eligible,
                CompactEntry {
                    id,
                    kind,
                    title,
                    body_size,
                    heat,
                    tier: heat_tier,
                    age_days,
                },
            ))
        })?
        .filter_map(|r| {
            if let Ok((eligible, entry)) = r {
                if eligible {
                    return Some(Ok(entry));
                }
            }
            None
        })
        .collect::<Result<Vec<_>, rusqlite::Error>>()?;

    Ok(candidates)
}

#[derive(Debug, Clone, Default)]
pub struct CompactOptions {
    pub dry_run: bool,
    pub tier: Option<String>,
}

/// Compact entries by gzipping their body to _archive/ and replacing with summary.
/// **NOTE:** This is a dry-run stub. Full implementation requires mutable DB access.
pub fn compact(
    _db: &rusqlite::Connection,
    vault_dir: &str,
    _options: &CompactOptions,
) -> Result<CompactResult> {
    ensure_archive_dir(vault_dir)?;

    Ok(CompactResult {
        candidates: 0,
        compacted: 0,
        bytes_reclaimed: 0,
        entries: Vec::new(),
    })
}

/// Restore a compacted entry's full body from the gzipped archive.
/// Returns decompressed body or None if no archive exists.
pub fn restore_compacted_body(vault_dir: &str, id: &str) -> Result<Option<String>> {
    let gz_path = archive_gz_path(vault_dir, id);
    if !gz_path.exists() {
        return Ok(None);
    }

    let compressed = fs::read(&gz_path)?;
    let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
    let mut body = String::new();
    use std::io::Read;
    decoder.read_to_string(&mut body)?;
    Ok(Some(body))
}

#[cfg(test)]
#[path = "compact_tests.rs"]
mod tests;
