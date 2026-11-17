// Re-exported so callers (e.g. sqlite.rs) keep importing `queries::entry_from_row`
// while the mapping itself lives in row_map.rs.
pub use super::row_map::entry_from_row;
use cv_engine::{Entry, ScoredEntry, SearchOptions};

use super::filters::filter_clause;
use super::fts_query;
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use zerocopy::AsBytes;

/// Column index of the score/distance appended by search JOINs — one past the 26 vault columns.
const COL_SCORE: usize = 26;

pub fn get_entry(conn: &Connection, id: &str) -> Result<Option<Entry>> {
    let result = conn
        .query_row(
            "SELECT id, kind, category, title, body, tags, meta, source, identity_key, tier, indexed, created_at, updated_at, expires_at, superseded_by, related_to, hit_count, recall_count, recall_sessions, last_recalled_at, heat_tier, summary_condensed, summary_keypoint, file_path, last_accessed_at, source_files FROM vault WHERE id = ?1",
            params![id],
            entry_from_row,
        )
        .optional()
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(result)
}

/// NULL rank (out-of-sync external-content index) reads as `None` -> unranked (0.0), so a
/// stale index degrades to unscored hits instead of crashing the search.
fn scored_from_fts(entry: Entry, rank: Option<f32>) -> ScoredEntry {
    let rank = rank.unwrap_or(0.0);
    ScoredEntry { fts_rank: Some(rank), vec_rank: None, score: (-rank).exp(), entry }
}

pub fn fts_search(conn: &Connection, query: &str, opts: SearchOptions) -> Result<Vec<ScoredEntry>> {
    let Some(match_query) = fts_query::all_words(query) else {
        return Ok(Vec::new());
    };
    let all_words = fts_match(conn, &match_query, &opts)?;
    if !all_words.is_empty() {
        return Ok(all_words);
    }
    // A question rarely has every word in the entry it means: fall back to any word, ranked.
    match fts_query::any_word(query) {
        Some(any_query) => fts_match(conn, &any_query, &opts),
        None => Ok(all_words),
    }
}

fn fts_match(conn: &Connection, match_query: &str, opts: &SearchOptions) -> Result<Vec<ScoredEntry>> {
    let sql = format!(
        // `fts.rank IS NULL` first in ORDER BY keeps unscored matches (stale external-content
        // index -> bm25 returns NULL) sorted LAST instead of SQLite's default NULLs-first.
        "SELECT v.id, v.kind, v.category, v.title, v.body, v.tags, v.meta, v.source, v.identity_key, v.tier, v.indexed, v.created_at, v.updated_at, v.expires_at, v.superseded_by, v.related_to, v.hit_count, v.recall_count, v.recall_sessions, v.last_recalled_at, v.heat_tier, v.summary_condensed, v.summary_keypoint, v.file_path, v.last_accessed_at, v.source_files, fts.rank FROM vault_fts fts JOIN vault v ON fts.rowid = v.rowid WHERE vault_fts MATCH ?1{} ORDER BY fts.rank IS NULL, fts.rank ASC LIMIT ? OFFSET ?",
        filter_clause(opts)
    );
    let mut stmt = conn.prepare(&sql)?;
    let scored = stmt
        .query_map(params![match_query, opts.limit, opts.offset], |row| {
            Ok(scored_from_fts(entry_from_row(row)?, row.get::<_, Option<f32>>(COL_SCORE)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(scored)
}

pub fn vec_search(conn: &Connection, embedding: &[f32], opts: SearchOptions) -> Result<Vec<ScoredEntry>> {
    // KNN over the vec0 table, then hydrate full rows from `vault`. The MATCH +
    // LIMIT constraint must sit directly on vault_vec, so it runs as a subquery.
    // Filters can't sit on the vec0 KNN itself, so fetch a larger candidate pool from
    // the KNN subquery, then apply the SearchOptions filters (and the final LIMIT) on
    // the hydrated `vault` rows in the outer query.
    let limit = opts.limit.max(1) as i64;
    let pool = (limit * 5).max(50);
    let sql = format!(
        "SELECT v.id, v.kind, v.category, v.title, v.body, v.tags, v.meta, v.source, v.identity_key, v.tier, v.indexed, v.created_at, v.updated_at, v.expires_at, v.superseded_by, v.related_to, v.hit_count, v.recall_count, v.recall_sessions, v.last_recalled_at, v.heat_tier, v.summary_condensed, v.summary_keypoint, v.file_path, v.last_accessed_at, v.source_files, m.distance \
        FROM (SELECT rowid, distance FROM vault_vec WHERE embedding MATCH ?1 ORDER BY distance LIMIT ?2) m \
        JOIN vault v ON v.rowid = m.rowid \
        WHERE 1=1{} \
        ORDER BY m.distance ASC LIMIT ?3",
        filter_clause(&opts)
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![embedding.as_bytes(), pool, limit], |row| {
            Ok((entry_from_row(row)?, row.get::<_, f64>(COL_SCORE)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let scored: Vec<ScoredEntry> = rows
        .into_iter()
        .map(|(entry, distance)| ScoredEntry {
            fts_rank: None,
            vec_rank: Some(distance as f32),
            // Monotonic decreasing in distance, bounded (0, 1]; mirrors the
            // exp-decay shape fts_search uses so RRF fuses comparable scores.
            score: 1.0 / (1.0 + distance as f32),
            entry,
        })
        .collect();

    Ok(scored)
}

pub fn fuse_results(fts: Vec<ScoredEntry>, vec: Vec<ScoredEntry>) -> Vec<ScoredEntry> {
    let tier_boosts = {
        let mut m = HashMap::new();
        m.insert("durable".to_string(), 1.5);
        m.insert("working".to_string(), 1.0);
        m.insert("ephemeral".to_string(), 0.7);
        m
    };

    let heat_boosts = {
        let mut m = HashMap::new();
        m.insert("hot".to_string(), 1.3);
        m.insert("warm".to_string(), 1.0);
        m.insert("cold".to_string(), 0.7);
        m
    };

    cv_engine::search::rrf_fuse(
        fts.iter().map(|se| (se.entry.clone(), se.score)).collect(),
        vec.iter().map(|se| (se.entry.clone(), se.score)).collect(),
        &tier_boosts,
        &heat_boosts,
    )
}
