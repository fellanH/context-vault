//! `SqliteBackend`'s access log (see `cv_engine::access`): hints shown go to `hint_log`,
//! reads to `access_log`, each row stamped in UTC ISO time so the two compare as text.

use anyhow::Result;
use async_trait::async_trait;
use cv_engine::access::AccessLog;
use rusqlite::{params, Connection};

use super::sqlite::SqliteBackend;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ','now')";

#[async_trait]
impl AccessLog for SqliteBackend {
    async fn log_hints(&self, ids: &[String], session: &str) -> Result<()> {
        let (db, ids, session) = (self.raw_db(), ids.to_vec(), session.to_string());
        let sql = format!("INSERT INTO hint_log (entry_id, session_id, shown_at) VALUES (?1, ?2, {NOW})");
        tokio::task::spawn_blocking(move || insert_all(&db.lock().unwrap(), &sql, &ids, &session)).await?
    }

    async fn log_reads(&self, ids: &[String], query: &str) -> Result<()> {
        let (db, ids, query) = (self.raw_db(), ids.to_vec(), query.to_string());
        let sql = format!("INSERT INTO access_log (entry_id, query, accessed_at) VALUES (?1, ?2, {NOW})");
        tokio::task::spawn_blocking(move || insert_all(&db.lock().unwrap(), &sql, &ids, &query)).await?
    }
}

/// One row per id, in one transaction, so a hook pays for a single write.
fn insert_all(conn: &Connection, sql: &str, ids: &[String], second: &str) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(sql)?;
        for id in ids {
            stmt.execute(params![id, second])?;
        }
    }
    tx.commit()?;
    Ok(())
}
