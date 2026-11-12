//! Situation reads over the `vault_situations` index (kept by triggers, see the reference schema).
//! A page is two steps: a keyset query over the index returning ids only, then the bodies of
//! those ids. Expired entries are filtered out (archived ones are not in the database at all).

use anyhow::{anyhow, Result};
use cv_engine::situations::{order_of, situations_of, valid_name};
use cv_engine::Entry;
use rusqlite::{params_from_iter, types::Value as Sql, Connection};
use serde_json::{json, Value};

use crate::queries::get_entry;
use crate::situation_cursor::{decode, encode, Cursor};

const FETCH_ROWS: usize = 30;

/// SQL for "this vault row has not expired", for a table aliased `v`.
pub(crate) const LIVE: &str = "(v.expires_at IS NULL OR v.expires_at > strftime('%Y-%m-%dT%H:%M:%fZ','now'))";

pub(crate) struct Key {
    pub(crate) id: String,
    pub(crate) ord: i64,
    pub(crate) updated: String,
}

pub fn list_situations(conn: &Connection) -> Result<Value> {
    let mut stmt = conn.prepare(&format!(
        "SELECT s.situation, COUNT(*) AS n FROM vault_situations s JOIN vault v ON v.id = s.entry_id \
         WHERE {LIVE} GROUP BY s.situation ORDER BY n DESC, s.situation ASC"
    ))?;
    let rows = stmt
        .query_map([], |r| Ok(json!({ "situation": r.get::<_, String>(0)?, "entries": r.get::<_, i64>(1)? })))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(json!({ "situations": rows }))
}

fn keys_sql(keyset: bool) -> String {
    let after = if keyset { "AND (s.ord > ?2 OR (s.ord = ?2 AND (s.updated < ?3 OR (s.updated = ?3 AND s.entry_id > ?4))))" } else { "" };
    format!(
        "SELECT s.entry_id, s.ord, s.updated FROM vault_situations s CROSS JOIN vault v ON v.id = s.entry_id \
         WHERE s.situation = ?1 {after} AND {LIVE} \
         ORDER BY s.ord ASC, s.updated DESC, s.entry_id ASC LIMIT {FETCH_ROWS}"
    )
}

fn page_keys(conn: &Connection, situation: &str, after: Option<Cursor>) -> Result<Vec<Key>> {
    let mut args = vec![Sql::Text(situation.to_string())];
    if let Some(c) = &after {
        args.extend([Sql::Integer(c.ord), Sql::Text(c.updated.clone()), Sql::Text(c.id.clone())]);
    }
    let mut stmt = conn.prepare(&keys_sql(after.is_some()))?;
    let rows = stmt.query_map(params_from_iter(args), |r| Ok(Key { id: r.get(0)?, ord: r.get(1)?, updated: r.get(2)? }))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// One entry of a page: the body is cut at `max_chars` and flagged when longer.
fn entry_json(entry: &Entry, len: usize, max_chars: usize) -> Value {
    let truncated = len > max_chars;
    let body: String = if truncated { entry.body.chars().take(max_chars).collect() } else { entry.body.clone() };
    let mut item = json!({
        "id": entry.id, "kind": entry.kind, "title": entry.title, "tags": entry.tags,
        "updated_at": entry.updated_at,
        "situations": situations_of(entry.meta.as_ref()),
        "situation_order": order_of(entry.meta.as_ref()),
        "body": body,
    });
    if truncated {
        item["truncated"] = json!(true);
        item["body_length"] = json!(len);
    }
    item
}

/// Bodies of the keyed entries until the page holds `max_chars` (at least one entry). Also returns how many
/// keys were consumed (an entry gone since the key query counts as consumed, so the cursor moves past it).
pub(crate) fn fill_page(conn: &Connection, keys: &[Key], max_chars: usize) -> Result<(Vec<Value>, usize)> {
    let (mut entries, mut used, mut consumed) = (Vec::new(), 0usize, 0usize);
    for key in keys {
        let Some(entry) = get_entry(conn, &key.id)? else {
            consumed += 1;
            continue;
        };
        let len = entry.body.chars().count();
        if !entries.is_empty() && used + len > max_chars {
            break;
        }
        entries.push(entry_json(&entry, len, max_chars));
        used += len.min(max_chars);
        consumed += 1;
    }
    Ok((entries, consumed))
}

/// One page of a situation in order (situation_order asc, updated desc, id) with bodies, until it holds
/// about `max_chars` of body text. A first entry longer than that is cut and marked truncated.
pub fn read_situation(conn: &Connection, situation: &str, cursor: Option<&str>, max_chars: usize) -> Result<Value> {
    if !valid_name(situation) {
        return Err(anyhow!("situation \"{situation}\" is not a valid name (lowercase-kebab, max 32 chars)"));
    }
    let after = cursor.filter(|c| !c.is_empty()).map(decode).transpose()?;
    let keys = page_keys(conn, situation, after)?;
    let (entries, consumed) = fill_page(conn, &keys, max_chars)?;
    let more = consumed < keys.len() || keys.len() == FETCH_ROWS;
    let last = consumed.checked_sub(1).map(|i| &keys[i]);
    let next_cursor = last.filter(|_| more).map(|k| encode(k.ord, &k.updated, &k.id));
    Ok(json!({ "situation": situation, "entries": entries, "next_cursor": next_cursor, "max_chars": max_chars }))
}
