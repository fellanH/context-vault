//! FTS5 token selection for read-only user vaults (`--vault-db`).
//!
//! Limits (documented in bench JSON via `fts_user_probe.limits_note`):
//! - Scans at most **200** indexed rows (title+body text), newest first.
//! - Considers at most **120** distinct normalized token candidates across the scan.
//! - Tokens must be **4–24** ASCII alphanumerics (after stripping punctuation), not a
//!   common English **stopword**, and not all digits.
//! - Each candidate is validated with `SELECT 1 FROM vault_fts WHERE vault_fts MATCH ? LIMIT 1`
//!   so empty FTS matches are never timed as if they were real work.

use std::collections::HashSet;

use anyhow::{bail, Result};
use rusqlite::Connection;
use serde::Serialize;

pub const MAX_TEXT_ROWS: usize = 200;
pub const MIN_TOKEN_LEN: usize = 4;
pub const MAX_TOKEN_LEN: usize = 24;
pub const MAX_CANDIDATES_TRIED: usize = 120;

#[derive(Debug, Clone, Serialize)]
pub struct FtsPickResult {
    pub token: String,
    pub rows_scanned: usize,
    pub candidates_tried: usize,
    pub strategy: &'static str,
    pub limits_note: &'static str,
}

const LIMITS_NOTE: &str = "Scans ≤200 indexed rows (newest first), ≤120 distinct candidates, token len 4–24 alnum, stopword-filtered; each candidate smoke-tested against vault_fts MATCH before use.";

static STOPWORDS: &[&str] = &[
    "that", "this", "with", "from", "your", "have", "will", "been", "were", "they",
    "their", "what", "when", "where", "which", "while", "about", "after", "before",
    "into", "through", "under", "above", "below", "between", "within", "without",
    "also", "just", "only", "even", "than", "then", "them", "these", "those", "there",
    "here", "some", "such", "very", "more", "most", "much", "many", "other", "another",
    "each", "every", "both", "either", "neither", "into", "onto", "upon", "over",
    "once", "again", "ever", "never", "always", "often", "sometimes", "being",
    "having", "doing", "could", "would", "should", "might", "must", "shall", "need",
    "make", "made", "like", "well", "back", "work", "used", "using", "based", "note",
    "tags", "kind", "body", "title", "date", "time", "http", "https", "www", "com",
    "null", "true", "false",
];

pub fn pick_user_fts_token(conn: &Connection) -> Result<FtsPickResult> {
    if let Some(r) = strategy_scan_rows(conn)? {
        return Ok(FtsPickResult {
            token: r.token,
            rows_scanned: r.rows_scanned,
            candidates_tried: r.candidates_tried,
            strategy: "first_match_multi_row_scan",
            limits_note: LIMITS_NOTE,
        });
    }
    if let Some(r) = strategy_fixed_fallbacks(conn)? {
        return Ok(FtsPickResult {
            token: r.token,
            rows_scanned: r.rows_scanned,
            candidates_tried: r.candidates_tried,
            strategy: "fixed_fallback_list",
            limits_note: LIMITS_NOTE,
        });
    }
    bail!(
        "Could not find any FTS5 MATCH token for this vault after scanning {MAX_TEXT_ROWS} rows and fixed fallbacks — vault may be empty, FTS not populated, or only stopwords."
    );
}

struct ScanHit {
    token: String,
    rows_scanned: usize,
    candidates_tried: usize,
}

fn strategy_scan_rows(conn: &Connection) -> Result<Option<ScanHit>> {
    let sql = "SELECT coalesce(title, '') || ' ' || coalesce(body, '') AS t \
               FROM vault WHERE indexed = 1 \
               ORDER BY COALESCE(updated_at, created_at) DESC \
               LIMIT ?1";
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([MAX_TEXT_ROWS as i64])?;
    let mut seen: HashSet<String> = HashSet::new();
    let mut rows_scanned = 0usize;
    let mut candidates_tried = 0usize;
    'outer: while let Some(row) = rows.next()? {
        let text: String = row.get(0)?;
        rows_scanned += 1;
        for cand in tokenize_candidates(&text) {
            if !seen.insert(cand.clone()) {
                continue;
            }
            candidates_tried += 1;
            if candidates_tried > MAX_CANDIDATES_TRIED {
                break 'outer;
            }
            if fts_match_smoke(conn, &cand)? {
                return Ok(Some(ScanHit {
                    token: cand,
                    rows_scanned,
                    candidates_tried,
                }));
            }
        }
    }
    Ok(None)
}

fn strategy_fixed_fallbacks(conn: &Connection) -> Result<Option<ScanHit>> {
    const FALLBACKS: &[&str] = &[
        "kind:decision",
        "kind:insight",
        "decision",
        "insight",
        "knowledge",
        "pattern",
        "vault",
        "context",
        "bench",
    ];
    for (candidates_tried, f) in FALLBACKS.iter().enumerate() {
        let n = candidates_tried + 1;
        if fts_match_smoke(conn, f)? {
            return Ok(Some(ScanHit {
                token: (*f).to_string(),
                rows_scanned: 0,
                candidates_tried: n,
            }));
        }
    }
    Ok(None)
}

fn fts_match_smoke(conn: &Connection, token: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM vault_fts WHERE vault_fts MATCH ? LIMIT 1")?;
    let mut rows = stmt.query([token])?;
    Ok(rows.next()?.is_some())
}

fn tokenize_candidates(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.split_whitespace() {
        let w = raw.trim_matches(|c: char| !c.is_alphanumeric());
        if let Some(t) = normalize_token(w) {
            out.push(t);
        }
    }
    out
}

fn normalize_token(w: &str) -> Option<String> {
    let lower = w.to_lowercase();
    let t: String = lower.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if t.len() < MIN_TOKEN_LEN || t.len() > MAX_TOKEN_LEN {
        return None;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if STOPWORDS.contains(&t.as_str()) {
        return None;
    }
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    use crate::bench_harness::sqlite_lane;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        let sql = std::fs::read_to_string(sqlite_lane::fixture_sql_path()).unwrap();
        conn.execute_batch(&sql).unwrap();
        conn.execute(
            "INSERT INTO vault (id, kind, category, title, body, tags, indexed, created_at, updated_at) VALUES \
             ('a', 'decision', 'knowledge', 'the and for', 'only stopwords here', 'x', 0, '2020-01-01', '2020-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO vault (id, kind, category, title, body, tags, indexed, created_at, updated_at) VALUES \
             ('b', 'decision', 'knowledge', 'xyzzyunique token', 'body text', 'x', 1, '2025-06-01', '2025-06-01')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn skips_stopword_heavy_first_row() {
        let conn = setup();
        let r = pick_user_fts_token(&conn).unwrap();
        assert_eq!(r.token, "xyzzyunique");
        assert!(r.rows_scanned >= 1);
        assert!(r.strategy == "first_match_multi_row_scan");
    }

    #[test]
    fn fixed_fallback_kind_decision() {
        let conn = Connection::open_in_memory().unwrap();
        let sql = std::fs::read_to_string(sqlite_lane::fixture_sql_path()).unwrap();
        conn.execute_batch(&sql).unwrap();
        conn.execute(
            "INSERT INTO vault (id, kind, category, title, body, tags, indexed) VALUES \
             ('z', 'decision', 'knowledge', 'a an the for', 'and or but nor', 'x', 1)",
            [],
        )
        .unwrap();
        let r = pick_user_fts_token(&conn).unwrap();
        assert!(
            r.strategy == "fixed_fallback_list",
            "expected fallback, got {:?}",
            r.strategy
        );
        assert_eq!(r.token, "kind:decision");
    }
}
