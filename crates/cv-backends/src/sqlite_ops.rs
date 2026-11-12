use cv_engine::{SaveInput, SaveOutput, HealthStatus, VaultStats};
use cv_vault::capture::WriteResult;
use anyhow::Result;
use rusqlite::ffi::sqlite3_auto_extension;
use rusqlite::{params, Connection};
use sqlite_vec::sqlite3_vec_init;
use std::sync::{Arc, Mutex, Once};
use std::collections::HashMap;
use zerocopy::AsBytes;

static VEC_INIT: Once = Once::new();

pub(super) fn register_sqlite_vec() {
    VEC_INIT.call_once(|| unsafe {
        #[allow(clippy::missing_transmute_annotations)]
        sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite3_vec_init as *const (),
        )));
    });
}

pub(super) fn is_zero_embedding(embedding: &[f32]) -> bool {
    embedding.is_empty() || embedding.iter().all(|&x| x == 0.0)
}

/// Insert or update a vault row that mirrors a markdown file on disk. The identity
/// column is `file_path` (UNIQUE): re-saving the same file (e.g. an entity keyed by
/// identity_key) upserts the existing row rather than creating a duplicate — matching
/// the TypeScript `db.ts` upsert-by-file_path invariant. The DB is an index over the
/// markdown files, so `wr` (the on-disk write result) is the source of truth for
/// `id`/`file_path`/`created_at`/`category`/`tier`.
pub async fn save_entry(
    db: Arc<Mutex<Connection>>,
    input: SaveInput,
    wr: WriteResult,
    embedding: Option<Vec<f32>>,
) -> Result<SaveOutput> {
    let tags = input.tags.as_ref().map(|t| serde_json::to_string(t).unwrap_or_default());
    let meta = input.meta.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default());
    let related_to = input
        .related_to
        .as_ref()
        .map(|r| serde_json::to_string(r).unwrap_or_default());
    // `supersedes` marks the OLD entries as replaced by this one; this row's own
    // `superseded_by` is never set here (a re-save keeps whatever it had).
    let supersedes = input.supersedes.clone().unwrap_or_default();

    let id = wr.id.clone();
    tokio::task::spawn_blocking({
        let db = db.clone();
        let id = id.clone();
        move || {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO vault (id, kind, category, title, body, tags, meta, source, file_path, identity_key, tier, indexed, created_at, updated_at, expires_at, related_to) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16) \
                 ON CONFLICT(file_path) DO UPDATE SET \
                   title=excluded.title, body=excluded.body, tags=excluded.tags, meta=excluded.meta, \
                   source=excluded.source, tier=excluded.tier, indexed=excluded.indexed, \
                   updated_at=excluded.updated_at, expires_at=excluded.expires_at, \
                   related_to=excluded.related_to",
                params![
                    &id, input.kind, wr.category, input.title, input.body, tags, meta,
                    input.source, wr.file_path, input.identity_key, wr.tier, 1,
                    wr.created_at, wr.updated_at, input.expires_at, related_to
                ],
            )?;
            for old in supersedes.iter().filter(|old| **old != id) {
                conn.execute("UPDATE vault SET superseded_by = ?1 WHERE id = ?2", params![&id, old])?;
            }
            // Resolve the rowid of the (possibly updated) row by its stable file_path,
            // then refresh the mirrored vector so re-saves don't leave a stale embedding.
            let rowid: i64 = conn.query_row(
                "SELECT rowid FROM vault WHERE file_path = ?1",
                params![wr.file_path],
                |row| row.get(0),
            )?;
            conn.execute("DELETE FROM vault_vec WHERE rowid = ?1", params![rowid])?;
            if let Some(emb) = embedding {
                if !is_zero_embedding(&emb) {
                    conn.execute(
                        "INSERT INTO vault_vec(rowid, embedding) VALUES (?1, ?2)",
                        params![rowid, emb.as_bytes()],
                    )?;
                }
            }
            Ok::<_, anyhow::Error>(())
        }
    })
    .await??;

    Ok(SaveOutput { id })
}

pub async fn init_schema(db: Arc<Mutex<Connection>>, dim: usize) -> Result<()> {
    tokio::task::spawn_blocking({
        let db = db.clone();
        move || {
            let conn = db.lock().unwrap();
            let schema = include_str!("../../../fixtures/bench/reference_schema.sql");
            conn.execute_batch(schema)?;
            super::sqlite_backfill::backfill_situations(&conn)?;
            conn.execute_batch(&format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS vault_vec USING vec0(embedding float[{dim}]);"
            ))?;
            Ok::<_, anyhow::Error>(())
        }
    })
    .await?
}

fn group_counts(conn: &Connection, sql: &str) -> anyhow::Result<HashMap<String, u64>> {
    let mut out = HashMap::new();
    let mut stmt = conn.prepare(sql)?;
    for result in stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)))? {
        let (key, count) = result?;
        out.insert(key, count);
    }
    Ok(out)
}

pub async fn compute_stats(db: Arc<Mutex<Connection>>) -> Result<VaultStats> {
    tokio::task::spawn_blocking({
        let db = db.clone();
        move || {
            let conn = db.lock().unwrap();
            let total: u64 = conn.query_row("SELECT COUNT(*) FROM vault", [], |row| row.get(0))?;
            let indexed: u64 = conn.query_row("SELECT COUNT(*) FROM vault WHERE indexed = 1", [], |row| row.get(0))?;
            let by_kind = group_counts(&conn, "SELECT kind, COUNT(*) FROM vault GROUP BY kind")?;
            let by_tier = group_counts(&conn, "SELECT tier, COUNT(*) FROM vault GROUP BY tier")?;

            Ok::<_, anyhow::Error>(VaultStats { total_entries: total, indexed_entries: indexed, by_kind, by_tier })
        }
    })
    .await?
}

pub async fn delete_entry(db: Arc<Mutex<Connection>>, id: &str) -> Result<()> {
    let id = id.to_string();
    tokio::task::spawn_blocking({
        let db = db.clone();
        move || {
            let conn = db.lock().unwrap();
            conn.execute("DELETE FROM vault WHERE id = ?1", params![&id])?;
            Ok::<_, anyhow::Error>(())
        }
    })
    .await??;
    Ok(())
}

pub async fn get_health(db: Arc<Mutex<Connection>>) -> HealthStatus {
    match compute_stats(db).await {
        Ok(_) => HealthStatus { ok: true, message: "vault operational".to_string() },
        Err(e) => HealthStatus { ok: false, message: format!("vault error: {}", e) },
    }
}
