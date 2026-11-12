//! The vault map: a deterministic overview of what the vault holds, with entry ids as addresses
//! (read the map first, then `get_context` by id). Cached in `vault_map`, regenerated lazily.
//!
//! Cost per generation is bounded whatever the vault's size: counts come from one GROUP BY, key
//! entries from at most 24 LIMITed queries, tags from the 400 most recently changed entries, and
//! only the first 400 characters of a body are read.

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::vault_map_areas::{build_areas, top_tags};
use crate::vault_map_query::{count_total, group_candidates, groups, recent, situated, Row};

pub const MAP_MAX_AGE_HOURS: i64 = 24;
pub const MAP_REFRESH_MIN_SECS: i64 = 60;
pub const MAP_CHANGE_RATIO: f64 = 0.02;
pub const MAP_CHANGE_ABS: i64 = 50;
const MAX_GROUPS_WITH_ENTRIES: usize = 24;
const MAX_GROUPS: usize = 60;

/// Build the map from the database. A pure read; see the module comment for the cost.
/// Returns the map and the entry count it was built at.
pub fn generate(conn: &Connection, now: DateTime<Utc>) -> Result<(Value, i64)> {
    let total = count_total(conn)?;
    let all = groups(conn)?;
    let shown = &all[..all.len().min(MAX_GROUPS)];
    let (situated_by_id, situated_rows) = situated(conn)?;
    let recent_rows = recent(conn)?;
    let tags = top_tags(&recent_rows);
    let tags_sampled_from = recent_rows.len();
    let candidates = group_candidates(conn, &shown[..shown.len().min(MAX_GROUPS_WITH_ENTRIES)])?;
    let mut pool: HashMap<String, Row> = HashMap::new();
    for row in situated_rows.into_iter().chain(recent_rows).chain(candidates) {
        pool.entry(row.id.clone()).or_insert(row);
    }
    let map = json!({
        "generated_at": now.to_rfc3339(),
        "totals": { "live": all.iter().map(|g| g.entries).sum::<i64>(), "groups": all.len() },
        "areas": build_areas(shown, &pool, &situated_by_id),
        "other_groups": all.len() - shown.len(),
        "tags": tags,
        "tags_sampled_from": tags_sampled_from,
        "situations": crate::situations::list_situations(conn)?["situations"].clone(),
    });
    Ok((map, total))
}

fn age(generated_at: &str, now: DateTime<Utc>) -> Option<Duration> {
    DateTime::parse_from_rfc3339(generated_at).ok().map(|t| now - t.with_timezone(&Utc))
}

/// Does a map built at `entries_at` entries, `generated_at`, need regenerating now?
pub fn is_stale(generated_at: &str, entries_at: i64, total: i64, now: DateTime<Utc>) -> bool {
    match age(generated_at, now) {
        Some(a) if a <= Duration::hours(MAP_MAX_AGE_HOURS) => {
            let delta = (total - entries_at).abs();
            delta > MAP_CHANGE_ABS || delta as f64 > entries_at as f64 * MAP_CHANGE_RATIO
        }
        _ => true,
    }
}

fn cached(conn: &Connection) -> Result<Option<(String, String, i64)>> {
    let sql = "SELECT map_json, generated_at, entries_at FROM vault_map WHERE id = 1";
    Ok(conn.query_row(sql, [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional()?)
}

fn store(conn: &Connection, map: &Value, entries_at: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO vault_map (id, map_json, generated_at, entries_at) VALUES (1, ?1, ?2, ?3) \
         ON CONFLICT(id) DO UPDATE SET map_json = excluded.map_json, generated_at = excluded.generated_at, entries_at = excluded.entries_at",
        params![map.to_string(), map["generated_at"].as_str().unwrap_or_default(), entries_at],
    )?;
    Ok(())
}

/// Is the cached map good for this request? A forced refresh within a minute of the last build also is.
fn usable(generated_at: &str, entries_at: i64, total: i64, force: bool, now: DateTime<Utc>) -> bool {
    if force {
        return age(generated_at, now).is_some_and(|a| a < Duration::seconds(MAP_REFRESH_MIN_SECS));
    }
    !is_stale(generated_at, entries_at, total, now)
}

/// The cached map, regenerated first when older than 24 h, when the entry count moved by more than 2%
/// or 50, or when `force` is set. `regenerated` tells which.
pub fn get(conn: &Connection, force: bool, now: DateTime<Utc>) -> Result<Value> {
    let total = count_total(conn)?;
    if let Some((text, generated_at, entries_at)) = cached(conn)? {
        let parsed = serde_json::from_str::<Value>(&text).ok();
        if let (true, Some(mut map)) = (usable(&generated_at, entries_at, total, force, now), parsed) {
            map["regenerated"] = json!(false);
            return Ok(map);
        }
    }
    let (mut map, entries_at) = generate(conn, now)?;
    store(conn, &map, entries_at)?;
    map["regenerated"] = json!(true);
    Ok(map)
}
