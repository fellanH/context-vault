//! One-time backfill of `vault_situations` for entries whose meta already carries situations (hand-written
//! frontmatter reindexed before the index existed). Guarded by a marker row in `index_state`, so the one scan
//! of the vault happens once; entries saved or reindexed afterwards are indexed by the triggers.

use anyhow::Result;
use rusqlite::Connection;

const MARKER: &str = "situations_backfilled";

const BACKFILL: &str = "INSERT OR IGNORE INTO vault_situations(situation, entry_id, ord, updated) \
    SELECT j.value, v.id, COALESCE(CAST(json_extract(v.meta, '$.situation_order') AS INTEGER), 100), \
           COALESCE(v.updated_at, v.created_at, '') \
      FROM vault v, json_each(CASE WHEN json_valid(v.meta) AND json_type(v.meta, '$.situations') = 'array' THEN v.meta ELSE '{}' END, '$.situations') j \
     WHERE v.meta LIKE '%\"situations\"%' AND j.type = 'text' AND length(j.value) BETWEEN 1 AND 32 \
       AND j.value NOT GLOB '*[^a-z0-9-]*' AND j.value NOT GLOB '-*' AND j.value NOT GLOB '*-' AND j.value NOT GLOB '*--*'";

pub(crate) fn backfill_situations(conn: &Connection) -> Result<()> {
    let done: i64 = conn.query_row("SELECT COUNT(*) FROM index_state WHERE key = ?1", [MARKER], |r| r.get(0))?;
    if done > 0 {
        return Ok(());
    }
    conn.execute_batch("BEGIN")?;
    let marked = |c: &Connection| c.execute("INSERT OR REPLACE INTO index_state(key, value) VALUES (?1, '1')", [MARKER]);
    let result = conn.execute_batch(BACKFILL).and_then(|_| marked(conn).map(|_| ()));
    conn.execute_batch(if result.is_ok() { "COMMIT" } else { "ROLLBACK" })?;
    Ok(result?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../../../fixtures/bench/reference_schema.sql")).unwrap();
        c
    }

    fn add(c: &Connection, id: &str, meta: &str) {
        c.execute("INSERT INTO vault (id, kind, body, meta, created_at, updated_at) VALUES (?1,'note','b',?2,'2026-01-01','2026-01-02')", [id, meta]).unwrap();
    }

    fn indexed(c: &Connection) -> Vec<(String, String, i64)> {
        let mut s = c.prepare("SELECT entry_id, situation, ord FROM vault_situations ORDER BY 1, 2").unwrap();
        s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().map(|x| x.unwrap()).collect()
    }

    #[test]
    fn triggers_skip_malformed_names_and_shapes() {
        let c = conn();
        add(&c, "mixed", r#"{"situations":["fresh","Bad Name","a--b","-x",5]}"#);
        add(&c, "str", r#"{"situations":"fresh"}"#);
        add(&c, "junk", "{not json situations");
        assert_eq!(indexed(&c), vec![("mixed".into(), "fresh".into(), 100)]);
    }

    #[test]
    fn backfills_marked_rows_once() {
        let c = conn();
        add(&c, "ok", r#"{"situations":["fresh","before-deploy"],"situation_order":7}"#);
        add(&c, "mixed", r#"{"situations":["fresh","Bad Name",5]}"#);
        add(&c, "str", r#"{"situations":"fresh"}"#);
        // rows that predate the index are not in it, and the marker is not set
        c.execute("DELETE FROM vault_situations", []).unwrap();
        c.execute("DELETE FROM index_state", []).unwrap();
        backfill_situations(&c).unwrap();
        let want = vec![("mixed".into(), "fresh".into(), 100), ("ok".into(), "before-deploy".into(), 7), ("ok".into(), "fresh".into(), 7)];
        assert_eq!(indexed(&c), want);
        // the second run is a no-op
        c.execute("DELETE FROM vault_situations", []).unwrap();
        backfill_situations(&c).unwrap();
        assert!(indexed(&c).is_empty());
    }
}
