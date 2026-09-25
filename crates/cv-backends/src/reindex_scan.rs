//! What an incremental reindex needs beyond the full walk: a watermark (when the last
//! successful reindex started), a per-file "changed since" test, and removal of rows
//! whose file is gone.
//!
//! Change detection uses max(mtime, ctime), not mtime alone: a rename (`schema apply`,
//! `context-vault move`) keeps mtime but bumps ctime, and a moved file that went
//! unnoticed would first lose its row to the orphan sweep and then never be re-added.

use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use cv_vault::WalkResult;
use rusqlite::{params, Connection, OptionalExtension};

const WATERMARK_KEY: &str = "reindex_started_at";

/// Files touched this close before the watermark are re-parsed anyway: a write that
/// lands while the previous walk is running must not be skipped by the next one.
const SLACK_SECS: f64 = 2.0;

pub fn now_secs() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

pub fn load_watermark(conn: &Connection) -> Result<Option<f64>> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM index_state WHERE key = ?1", params![WATERMARK_KEY], |r| r.get(0))
        .optional()?;
    Ok(value.and_then(|v| v.parse().ok()))
}

pub fn store_watermark(conn: &Connection, started_at: f64) -> Result<()> {
    conn.execute(
        "INSERT INTO index_state(key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![WATERMARK_KEY, started_at.to_string()],
    )?;
    Ok(())
}

/// True when the file changed at or after `since` (minus slack), or can't be stat'ed
/// (re-parsing is the safe side: the parse step skips unreadable files itself).
pub fn changed_since(path: &str, since: f64) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return true };
    let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok());
    let mut latest = modified.map(|d| d.as_secs_f64()).unwrap_or(f64::MAX);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        latest = latest.max(meta.ctime() as f64 + meta.ctime_nsec() as f64 / 1e9);
    }
    latest >= since - SLACK_SECS
}

/// Outcome of the orphan sweep: rows removed, or the reason it refused to run.
pub enum Prune {
    Removed(u64),
    Refused(String),
}

/// Delete rows (and their vectors) whose file is no longer on disk.
///
/// Refuses, rather than deletes, when the evidence looks like a missing vault
/// rather than deleted files: the walk found nothing, or more than half the rows
/// would go. An unmounted or mistyped vault dir must not wipe the index.
pub fn prune_orphans(conn: &Connection, seen: &HashSet<String>) -> Result<Prune> {
    let (orphans, total) = find_orphans(conn, seen)?;
    if orphans.is_empty() {
        return Ok(Prune::Removed(0));
    }
    if seen.is_empty() || orphans.len() * 2 > total {
        return Ok(Prune::Refused(format!(
            "{} of {} indexed files are missing on disk — not removing them (vault dir wrong or unmounted?)",
            orphans.len(),
            total
        )));
    }
    for rowid in &orphans {
        conn.execute("DELETE FROM vault_vec WHERE rowid = ?1", params![rowid])?;
        conn.execute("DELETE FROM vault WHERE rowid = ?1", params![rowid])?;
    }
    Ok(Prune::Removed(orphans.len() as u64))
}

/// Rowids whose file is neither in the walk nor on disk, plus the number of rows with a path.
fn find_orphans(conn: &Connection, seen: &HashSet<String>) -> Result<(Vec<i64>, usize)> {
    let mut stmt = conn.prepare("SELECT rowid, file_path FROM vault WHERE file_path IS NOT NULL AND file_path != ''")?;
    let rows: Vec<(i64, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
    let orphans = rows
        .iter()
        .filter(|(_, p)| !seen.contains(p) && !Path::new(p).exists())
        .map(|(rowid, _)| *rowid)
        .collect();
    Ok((orphans, rows.len()))
}

/// Files to re-parse: all of them without a watermark, else only those changed since it.
pub fn select_changed(files: &[WalkResult], since: Option<f64>) -> Vec<WalkResult> {
    match since {
        Some(t) => files.iter().filter(|f| changed_since(&f.file_path, t)).cloned().collect(),
        None => files.to_vec(),
    }
}

/// Sweep rows for vanished files and advance the watermark, in one transaction.
pub fn finish(conn: &mut Connection, files: &[WalkResult], started_at: f64) -> Result<(u64, Option<String>)> {
    let seen: HashSet<String> = files.iter().map(|f| f.file_path.clone()).collect();
    let tx = conn.transaction()?;
    let prune = prune_orphans(&tx, &seen)?;
    store_watermark(&tx, started_at)?;
    tx.commit()?;
    Ok(match prune {
        Prune::Removed(k) => (k, None),
        Prune::Refused(why) => (0, Some(why)),
    })
}
