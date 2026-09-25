//! Maps a `vault` result row onto an `Entry`.
//!
//! The projection is 26 columns wide (indices 0..=25), matching the SELECT lists
//! in `queries.rs`. The mapping is split into contiguous-range helpers so no single
//! function's branch count (one per fallible `?`) approaches the complexity ceiling —
//! the width is bound by the schema, not by control flow.

use cv_engine::Entry;
use serde_json::Value;

fn json_opt<T: serde::de::DeserializeOwned>(
    row: &rusqlite::Row,
    idx: usize,
) -> rusqlite::Result<Option<T>> {
    row.get::<_, Option<String>>(idx)
        .map(|opt| opt.and_then(|s| serde_json::from_str(&s).ok()))
}

/// Columns 0..=6: id, kind, category, title, body, tags, meta.
type Content = (String, String, String, Option<String>, String, Vec<String>, Option<Value>);
fn map_content(row: &rusqlite::Row) -> rusqlite::Result<Content> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        json_opt::<Vec<String>>(row, 5)?.unwrap_or_default(),
        json_opt(row, 6)?,
    ))
}

/// Columns 7..=13: source, identity_key, tier, indexed, created_at, updated_at, expires_at.
type Classification = (Option<String>, Option<String>, String, bool, String, String, Option<String>);
fn map_classification(row: &rusqlite::Row) -> rusqlite::Result<Classification> {
    Ok((
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get::<_, i32>(10)? != 0,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
    ))
}

/// Columns 14..=20: superseded_by, related_to, hit_count, recall_count, recall_sessions,
/// last_recalled_at, heat_tier.
type Signals = (Option<String>, Option<Vec<String>>, i64, i64, i64, Option<String>, Option<String>);
fn map_signals(row: &rusqlite::Row) -> rusqlite::Result<Signals> {
    Ok((
        row.get(14)?,
        json_opt(row, 15)?,
        row.get(16)?,
        row.get(17)?,
        row.get(18)?,
        row.get(19)?,
        row.get(20)?,
    ))
}

/// Columns 21..=25: summary_condensed, summary_keypoint, file_path, last_accessed_at, source_files.
type Provenance = (Option<String>, Option<String>, Option<String>, Option<String>, Option<Value>);
fn map_provenance(row: &rusqlite::Row) -> rusqlite::Result<Provenance> {
    Ok((
        row.get(21)?,
        row.get(22)?,
        row.get(23)?,
        row.get(24)?,
        json_opt(row, 25)?,
    ))
}

pub fn entry_from_row(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    let (id, kind, category, title, body, tags, meta) = map_content(row)?;
    let (source, identity_key, tier, indexed, created_at, updated_at, expires_at) =
        map_classification(row)?;
    let (superseded_by, related_to, hit_count, recall_count, recall_sessions, last_recalled_at, heat_tier) =
        map_signals(row)?;
    let (summary_condensed, summary_keypoint, file_path, last_accessed_at, source_files) =
        map_provenance(row)?;

    Ok(Entry {
        id,
        kind,
        category,
        title,
        body,
        tags,
        meta,
        source,
        identity_key,
        tier,
        indexed,
        created_at,
        updated_at,
        expires_at,
        superseded_by,
        supersedes: None,
        related_to,
        hit_count,
        recall_count,
        recall_sessions,
        last_recalled_at,
        heat_tier,
        summary_condensed,
        summary_keypoint,
        file_path,
        last_accessed_at,
        source_files,
    })
}
