//! The reads behind the vault map. Each is bounded whatever the vault's size.

use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::HashMap;

use crate::situations::LIVE;

const SITUATION_SAMPLE: i64 = 300;
const RECENT_SAMPLE: i64 = 400;
const PER_GROUP_CANDIDATES: i64 = 16;
/// Body characters read per entry (for the one-line summary).
const COLUMNS: &str = "v.id, v.kind, v.category, v.title, v.tags, COALESCE(v.updated_at, v.created_at), substr(v.body, 1, 400)";

pub struct Row {
    pub id: String,
    pub kind: String,
    pub category: String,
    pub title: Option<String>,
    pub tags: Option<String>,
    pub updated: String,
    pub head: String,
}

pub struct Group {
    pub category: String,
    pub kind: String,
    pub entries: i64,
}

fn row_at(r: &rusqlite::Row, at: usize) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(at)?,
        kind: r.get(at + 1)?,
        category: r.get(at + 2)?,
        title: r.get(at + 3)?,
        tags: r.get(at + 4)?,
        updated: r.get::<_, Option<String>>(at + 5)?.unwrap_or_default(),
        head: r.get::<_, Option<String>>(at + 6)?.unwrap_or_default(),
    })
}

/// Live entries per category and kind, biggest first.
pub fn groups(conn: &Connection) -> Result<Vec<Group>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT v.category, v.kind, COUNT(*) AS n FROM vault v WHERE {LIVE} GROUP BY v.category, v.kind ORDER BY n DESC, v.kind ASC"
    ))?;
    let rows = stmt.query_map([], |r| Ok(Group { category: r.get(0)?, kind: r.get(1)?, entries: r.get(2)? }))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Entry id -> the situations it carries.
pub type SituationsById = HashMap<String, Vec<String>>;

/// The recently changed entries that carry a situation: id -> situations, plus their rows.
pub fn situated(conn: &Connection) -> Result<(SituationsById, Vec<Row>)> {
    let mut stmt = conn.prepare(&format!(
        "SELECT s.situation, {COLUMNS} FROM vault_situations s JOIN vault v ON v.id = s.entry_id WHERE {LIVE} \
         ORDER BY COALESCE(v.updated_at, v.created_at) DESC LIMIT {SITUATION_SAMPLE}"
    ))?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, row_at(r, 1)?)))?;
    let mut of: HashMap<String, Vec<String>> = HashMap::new();
    let mut found = Vec::new();
    for item in rows {
        let (situation, row) = item?;
        of.entry(row.id.clone()).or_default().push(situation);
        found.push(row);
    }
    Ok((of, found))
}

/// The most recently changed live entries (the tag sample and extra key-entry candidates).
pub fn recent(conn: &Connection) -> Result<Vec<Row>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM vault v WHERE {LIVE} ORDER BY v.updated_at DESC LIMIT {RECENT_SAMPLE}"))?;
    let rows = stmt.query_map([], |r| row_at(r, 0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The newest entries of each given group (served by `idx_vault_kind_cat_created`).
pub fn group_candidates(conn: &Connection, groups: &[Group]) -> Result<Vec<Row>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM vault v WHERE v.kind = ?1 AND v.category = ?2 AND {LIVE} ORDER BY v.created_at DESC LIMIT {PER_GROUP_CANDIDATES}"
    ))?;
    let mut out = Vec::new();
    for group in groups {
        for row in stmt.query_map(params![group.kind, group.category], |r| row_at(r, 0))? {
            out.push(row?);
        }
    }
    Ok(out)
}

pub fn count_total(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM vault", [], |r| r.get(0))?)
}
