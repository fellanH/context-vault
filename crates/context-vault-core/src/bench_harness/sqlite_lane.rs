use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use super::fts_token;
use super::{corpus, DiskMode, WorkloadMix};

pub const SQL_COUNT_INDEXED: &str = "SELECT COUNT(*) AS c FROM vault WHERE indexed = 1";
pub const SQL_LIST_PAGE_100: &str =
    "SELECT id, title FROM vault WHERE indexed = 1 ORDER BY created_at DESC LIMIT 100 OFFSET 0";
pub const SQL_FTS_MATCH_HYBRID: &str = "\
SELECT v.id, v.title FROM vault v \
WHERE v.indexed = 1 AND v.rowid IN (\
  SELECT rowid FROM vault_fts WHERE vault_fts MATCH 'hybrid'\
) LIMIT 20";

#[derive(Debug, Serialize)]
pub struct SqliteLane {
    pub db_kind: &'static str,
    pub disk_mode: DiskMode,
    pub vault_source: &'static str,
    pub sql_count_indexed_ms: f64,
    pub sql_list_page_100_ms: f64,
    pub sql_fts_match_ms: f64,
    pub sql_bulk_insert_ms: Option<f64>,
    pub fts_match_row_count: usize,
    pub fts_query_used: String,
    pub fts_user_probe: Option<fts_token::FtsPickResult>,
    pub sql_probes: SqlProbes,
    pub mode_a_rg_equivalent: &'static str,
    pub filter_parity_note: &'static str,
}

#[derive(Debug, Serialize)]
pub struct SqlProbes {
    pub count_indexed: &'static str,
    pub list_page_100: &'static str,
    pub fts_match: String,
    pub bulk_insert_template: &'static str,
}

pub fn fixture_sql_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bench/reference_schema.sql")
}

pub fn open_user_vault(path: &Path) -> Result<Connection> {
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
        | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
    Connection::open_with_flags(path, flags)
        .with_context(|| format!("open vault db (read-only) {}", path.display()))
}

pub fn materialize_and_seed(
    disk_mode: DiskMode,
    seed: u64,
    entries: u32,
) -> Result<(Connection, Option<PathBuf>)> {
    match disk_mode {
        DiskMode::A => {
            let conn = Connection::open_in_memory().context("open in-memory sqlite")?;
            apply_fixture(&conn)?;
            apply_pragmas_for_mode(&conn, disk_mode)?;
            seed_rows(&conn, seed, entries)?;
            Ok((conn, None))
        }
        DiskMode::B | DiskMode::C => {
            let dir = tempfile::tempdir().context("tempdir for file-backed bench db")?;
            let db_path = dir.path().join("bench.db");
            let conn = Connection::open(&db_path)
                .with_context(|| format!("open sqlite {}", db_path.display()))?;
            apply_fixture(&conn)?;
            apply_pragmas_for_mode(&conn, disk_mode)?;
            seed_rows(&conn, seed, entries)?;
            Ok((conn, Some(db_path)))
        }
    }
}

fn apply_pragmas_for_mode(conn: &Connection, mode: DiskMode) -> Result<()> {
    match mode {
        DiskMode::A => {
            conn.execute_batch("PRAGMA journal_mode = MEMORY;\nPRAGMA synchronous = OFF;\n")?;
        }
        DiskMode::B => {
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;\nPRAGMA synchronous = FULL;\nPRAGMA busy_timeout = 5000;\n",
            )?;
        }
        DiskMode::C => {
            conn.execute_batch(
                "PRAGMA journal_mode = DELETE;\nPRAGMA synchronous = FULL;\nPRAGMA busy_timeout = 8000;\n",
            )?;
        }
    }
    Ok(())
}

fn apply_fixture(conn: &Connection) -> Result<()> {
    let sql = fs::read_to_string(fixture_sql_path())
        .with_context(|| format!("read {}", fixture_sql_path().display()))?;
    conn.execute_batch(&sql).context("apply reference_schema.sql")?;
    Ok(())
}

fn seed_rows(conn: &Connection, seed: u64, entries: u32) -> Result<()> {
    let mut stmt = conn.prepare_cached(
        "INSERT INTO vault (id, kind, category, title, body, tags, indexed) VALUES (?1, 'decision', 'knowledge', ?2, ?3, ?4, 1)",
    )?;
    for i in 0..entries {
        let (id, title, body) = corpus::vault_seed_row(seed, i);
        stmt.execute((&id, &title, &body, "bench,generated"))?;
    }
    Ok(())
}

pub fn run_fixture_probes(
    conn: &Connection,
    workload: WorkloadMix,
    disk_mode: DiskMode,
    db_path: Option<&Path>,
) -> Result<SqliteLane> {
    let db_kind = if db_path.is_none() && matches!(disk_mode, DiskMode::A) {
        "memory"
    } else {
        "file"
    };

    let t0 = Instant::now();
    let _: i64 = conn.query_row(SQL_COUNT_INDEXED, [], |r| r.get(0))?;
    let sql_count_indexed_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t0 = Instant::now();
    let mut stmt = conn.prepare(SQL_LIST_PAGE_100)?;
    let rows: Vec<(String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);
    let sql_list_page_100_ms = t0.elapsed().as_secs_f64() * 1000.0;
    if rows.len() > 100 {
        anyhow::bail!("list page returned more than 100 rows");
    }

    let t0 = Instant::now();
    let mut stmt = conn.prepare(SQL_FTS_MATCH_HYBRID)?;
    let fts_rows: Vec<(String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let fts_match_row_count = fts_rows.len();
    drop(stmt);
    let sql_fts_match_ms = t0.elapsed().as_secs_f64() * 1000.0;
    if fts_match_row_count == 0 && workload != WorkloadMix::WriteHeavy {
        anyhow::bail!("FTS MATCH 'hybrid' returned 0 rows — seed titles must include token hybrid");
    }

    let sql_bulk_insert_ms = run_bulk_insert_probe(conn, workload)?;

    Ok(SqliteLane {
        db_kind,
        disk_mode,
        vault_source: "fixture+seed",
        sql_count_indexed_ms,
        sql_list_page_100_ms,
        sql_fts_match_ms,
        sql_bulk_insert_ms,
        fts_match_row_count,
        fts_query_used: "hybrid".to_string(),
        fts_user_probe: None,
        sql_probes: SqlProbes {
            count_indexed: SQL_COUNT_INDEXED,
            list_page_100: SQL_LIST_PAGE_100,
            fts_match: SQL_FTS_MATCH_HYBRID.to_string(),
            bulk_insert_template: "INSERT INTO vault (...) batched in transaction (fixture DB only)",
        },
        mode_a_rg_equivalent: "rg --no-heading --glob '*.md' 'hybrid' <corpus-root>/knowledge/decisions  # substring on markdown; NOT FTS5 tokenization/MATCH ranking",
        filter_parity_note: "COUNT + list page match scripts/benchmark-hotpath.ts SQL strings; FTS branch is FTS5-only slice (no embeddings, no sqlite-vec).",
    })
}

fn run_bulk_insert_probe(conn: &Connection, workload: WorkloadMix) -> Result<Option<f64>> {
    let ms = match workload {
        WorkloadMix::ReadHeavy => {
            let t0 = Instant::now();
            conn.execute(
                "INSERT INTO vault (id, kind, category, title, body, tags, indexed) VALUES ('bench-write-probe', 'insight', 'knowledge', 'write probe', 'body hybrid', 'bench', 1)",
                [],
            )?;
            t0.elapsed().as_secs_f64() * 1000.0
        }
        WorkloadMix::Balanced | WorkloadMix::WriteHeavy => {
            let t0 = Instant::now();
            let tx = conn.unchecked_transaction()?;
            let n = if matches!(workload, WorkloadMix::WriteHeavy) {
                200
            } else {
                50
            };
            for j in 0..n {
                let id = format!("bench-bulk-{j:04}");
                tx.execute(
                    "INSERT INTO vault (id, kind, category, title, body, tags, indexed) VALUES (?1, 'insight', 'knowledge', ?2, ?3, 'bench', 1)",
                    (&id, &format!("bulk {j}"), &format!("bulk body {j} token hybrid")),
                )?;
            }
            tx.commit()?;
            t0.elapsed().as_secs_f64() * 1000.0
        }
    };
    Ok(Some(ms))
}

pub fn run_user_vault_read_probes(
    conn: &Connection,
    _workload: WorkloadMix,
    disk_mode: DiskMode,
) -> Result<SqliteLane> {
    let fts_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table','virtual') AND name='vault_fts'",
        [],
        |r| r.get(0),
    )?;
    if fts_exists == 0 {
        anyhow::bail!("vault_fts missing on this database — cannot run FTS bench probe");
    }
    let db_kind = "file";
    let pick = fts_token::pick_user_fts_token(conn)?;
    let token = pick.token.clone();
    let fts_sql = "SELECT v.id, v.title FROM vault v WHERE v.indexed = 1 AND v.rowid IN (\
           SELECT rowid FROM vault_fts WHERE vault_fts MATCH ?\
         ) LIMIT 20"
        .to_string();

    let t0 = Instant::now();
    let _: i64 = conn.query_row(SQL_COUNT_INDEXED, [], |r| r.get(0))?;
    let sql_count_indexed_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t0 = Instant::now();
    let mut stmt = conn.prepare(SQL_LIST_PAGE_100)?;
    let rows: Vec<(String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);
    let sql_list_page_100_ms = t0.elapsed().as_secs_f64() * 1000.0;
    if rows.len() > 100 {
        anyhow::bail!("list page returned more than 100 rows");
    }

    let t0 = Instant::now();
    let mut stmt = conn.prepare(&fts_sql)?;
    let fts_rows: Vec<(String, Option<String>)> = stmt
        .query_map([&token], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let fts_match_row_count = fts_rows.len();
    drop(stmt);
    let sql_fts_match_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let sql_bulk_insert_ms: Option<f64> = None;

    Ok(SqliteLane {
        db_kind,
        disk_mode,
        vault_source: "user-copy-read-only",
        sql_count_indexed_ms,
        sql_list_page_100_ms,
        sql_fts_match_ms,
        sql_bulk_insert_ms,
        fts_match_row_count,
        fts_query_used: token.clone(),
        fts_user_probe: Some(pick),
        sql_probes: SqlProbes {
            count_indexed: SQL_COUNT_INDEXED,
            list_page_100: SQL_LIST_PAGE_100,
            fts_match: fts_sql,
            bulk_insert_template: "skipped on user vault (read-only connection)",
        },
        mode_a_rg_equivalent: "rg --no-heading --glob '*.md' <token> <vault-md-tree>  # approximate disk grep lane vs SQLite FTS",
        filter_parity_note: "COUNT/list identical to hotpath script; FTS uses multi-row token scan + fallbacks (see fts_user_probe) — each candidate smoke-tested on vault_fts before timing.",
    })
}
