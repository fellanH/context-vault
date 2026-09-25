//! Walk the vault markdown tree and rebuild the SQLite index from disk. Each `.md`
//! file's frontmatter is parsed into a row, upserted by `id` (falling back to the
//! DB's existing id for that `file_path` when frontmatter carries none), so re-runs
//! are idempotent. This makes the DB a derived index over the markdown files
//! (matching the TS `reindex()` contract) and returns the real count.
//!
//! Incremental by default: only files changed since the last successful reindex
//! started are re-parsed, and rows whose file is gone are removed (see reindex_scan).
//! `full` re-parses everything.
//!
//! Note (Phase 1): rows are rebuilt but vectors are NOT regenerated here — that needs
//! the async embedder and lands with the CLI `reindex` in a later phase. FTS is kept
//! in sync via the vault_ai/vault_au triggers on every row write, so keyword search
//! stays correct; hybrid degrades to FTS for un-embedded rows.

use anyhow::Result;
use chrono::Utc;
use cv_engine::ReindexSummary;
use cv_vault::{parse_frontmatter, walk_dir, VaultTaxonomy, WalkResult};
use rusqlite::{named_params, params, Connection, Transaction};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::reindex_scan::{finish, load_watermark, now_secs, select_changed};
use ulid::Ulid;

const VALID_TIERS: [&str; 3] = ["ephemeral", "working", "durable"];
const VALID_HEAT_TIERS: [&str; 3] = ["hot", "warm", "cold"];
// Commit in bounded batches rather than one transaction spanning the whole vault: a
// single transaction that fires the vault_ai/vault_au triggers tens of thousands of
// times against an already-populated vault_fts (an external-content FTS5 table) was
// observed to corrupt its shadow tables ("database disk image is malformed" /
// "Content in the virtual table is corrupt") once real vault size crossed ~80k rows
// -- reproduced against a full copy of the production vault, absent on a
// from-scratch DB of the same size. Batching keeps each transaction's FTS5 write
// volume bounded and sidesteps it, while upserts stay idempotent so a mid-run
// failure only needs a re-run, not a rollback of everything reindexed so far.
const BATCH_SIZE: usize = 1000;

struct FileRow {
    id: String,
    kind: String,
    category: String,
    title: Option<String>,
    body: String,
    tags: Option<String>,
    source: Option<String>,
    identity_key: Option<String>,
    tier: String,
    heat_tier: Option<String>,
    created: String,
    updated: String,
    expires_at: Option<String>,
}

pub async fn reindex_entries(db: Arc<Mutex<Connection>>, vault_dir: String, full: bool) -> Result<ReindexSummary> {
    let start = std::time::Instant::now();
    let mut summary = tokio::task::spawn_blocking(move || run_reindex_blocking(db, vault_dir, full)).await??;
    summary.duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(summary)
}

fn run_reindex_blocking(db: Arc<Mutex<Connection>>, vault_dir: String, full: bool) -> Result<ReindexSummary> {
    // Taken before the walk: anything written during this run is newer than it and
    // gets picked up by the next one.
    let started_at = now_secs();
    let files = if Path::new(&vault_dir).is_dir() { walk_dir(&vault_dir)? } else { Vec::new() };
    let taxonomy = VaultTaxonomy::load(&vault_dir);
    let mut conn = db.lock().unwrap();
    let since = if full { None } else { load_watermark(&conn)? };
    let changed = select_changed(&files, since);

    let mut n: u64 = 0;
    for chunk in changed.chunks(BATCH_SIZE) {
        n += reindex_batch(&mut conn, chunk, &taxonomy)?;
    }

    let (rows_removed, prune_refused) = finish(&mut conn, &files, started_at)?;
    Ok(ReindexSummary { entries_reindexed: n, duration_ms: 0.0, files_seen: files.len() as u64, rows_removed, prune_refused })
}

fn reindex_batch(conn: &mut Connection, chunk: &[WalkResult], taxonomy: &VaultTaxonomy) -> Result<u64> {
    let tx = conn.transaction()?;
    let mut n: u64 = 0;
    for wr in chunk {
        if reindex_one_file(&tx, wr, taxonomy)? {
            n += 1;
        }
    }
    tx.commit()?;
    Ok(n)
}

/// Parse and upsert one file. Returns `Ok(false)` (not a failure) when the file
/// can't be read -- e.g. deleted between the walk and this pass.
fn reindex_one_file(tx: &Transaction, wr: &WalkResult, taxonomy: &VaultTaxonomy) -> Result<bool> {
    let raw = match std::fs::read_to_string(&wr.file_path) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let (fm, body) = parse_frontmatter(&raw);
    let row = build_row(&fm, body, wr, taxonomy, tx)?;
    upsert_row(tx, &row, &wr.file_path)?;
    Ok(true)
}

fn build_row(
    fm: &serde_json::Map<String, serde_json::Value>,
    body: String,
    wr: &WalkResult,
    taxonomy: &VaultTaxonomy,
    tx: &Transaction,
) -> Result<FileRow> {
    let get_str = |k: &str| fm.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());

    // kind: frontmatter wins; else the leaf directory name under the category.
    let kind = get_str("kind").unwrap_or_else(|| {
        Path::new(&wr.rel_dir).file_name().and_then(|s| s.to_str()).unwrap_or("note").to_string()
    });
    let category = taxonomy.category_for(&kind).to_string();

    // A file with no persisted `id` in frontmatter would otherwise mint a fresh
    // ULID on every reindex pass, which never matches the ON CONFLICT(id) target
    // in upsert_row and so re-attempts a plain INSERT that collides with the
    // file_path UNIQUE index on the row from the last run. Reuse the DB's existing
    // id for this file_path (if any) so reindexing stays idempotent for id-less
    // files instead of tripping that constraint every time.
    let id = get_str("id").unwrap_or_else(|| {
        tx.query_row("SELECT id FROM vault WHERE file_path = ?1", params![wr.file_path], |row| {
            row.get::<_, String>(0)
        })
        .unwrap_or_else(|_| Ulid::new().to_string())
    });

    let created = get_str("created").unwrap_or_else(|| Utc::now().to_rfc3339());
    let updated = get_str("updated").unwrap_or_else(|| created.clone());
    let (tier, heat_tier) = resolve_tiers(get_str("tier"), get_str("heat_tier"));

    Ok(FileRow {
        id,
        kind,
        category,
        title: get_str("title"),
        body,
        tags: fm.get("tags").map(|v| v.to_string()),
        source: get_str("source"),
        identity_key: get_str("identity_key"),
        tier,
        heat_tier,
        created,
        updated,
        expires_at: get_str("expires_at"),
    })
}

/// Some older captures (tool-call events) wrote a heat_tier value ("cold") into the
/// `tier` field instead of `heat_tier` -- `tier` has its own
/// CHECK(IN ('ephemeral','working','durable')), so that value fails the constraint
/// outright. Route a hot/warm/cold value to heat_tier (where it belongs) and fall
/// tier itself back to the schema default so a mislabeled file can't abort reindex.
fn resolve_tiers(raw_tier: Option<String>, raw_heat_tier: Option<String>) -> (String, Option<String>) {
    let tier = raw_tier
        .as_deref()
        .filter(|t| VALID_TIERS.contains(t))
        .unwrap_or("working")
        .to_string();
    let heat_tier = raw_heat_tier
        .filter(|t| VALID_HEAT_TIERS.contains(&t.as_str()))
        .or_else(|| raw_tier.filter(|t| VALID_HEAT_TIERS.contains(&t.as_str())));
    (tier, heat_tier)
}

// Conflict target is `id` (the primary key), not `file_path`: a file that moved on
// disk (e.g. `context-vault schema apply` after a vault.toml edit) keeps its
// frontmatter id but changes file_path, so file_path can never find the existing
// row to update -- it would instead attempt a fresh INSERT that collides with the
// id's PRIMARY KEY constraint.
fn upsert_row(tx: &Transaction, row: &FileRow, file_path: &str) -> Result<()> {
    match insert_row(tx, row, file_path) {
        Ok(_) => Ok(()),
        // Belt-and-braces: the id lookup in build_row should already prevent this,
        // but if some other row still holds this file_path under a different id,
        // fall back to updating that row by file_path instead of aborting -- and
        // thereby rolling back -- the entire reindex over one bad row. Any other
        // DB error still propagates.
        Err(rusqlite::Error::SqliteFailure(err, _)) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
            eprintln!("[context-vault] reindex: file_path conflict for {file_path}, updating existing row by path");
            update_row_by_file_path(tx, row, file_path)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

fn insert_row(tx: &Transaction, row: &FileRow, file_path: &str) -> rusqlite::Result<usize> {
    tx.execute(
        "INSERT INTO vault (id, kind, category, title, body, tags, source, file_path, identity_key, tier, heat_tier, indexed, created_at, updated_at, expires_at) \
         VALUES (:id, :kind, :category, :title, :body, :tags, :source, :file_path, :identity_key, :tier, :heat_tier, :indexed, :created_at, :updated_at, :expires_at) \
         ON CONFLICT(id) DO UPDATE SET \
           kind=excluded.kind, category=excluded.category, title=excluded.title, \
           body=excluded.body, tags=excluded.tags, source=excluded.source, \
           file_path=excluded.file_path, tier=excluded.tier, heat_tier=excluded.heat_tier, \
           updated_at=excluded.updated_at, expires_at=excluded.expires_at",
        named_params! {
            ":id": row.id, ":kind": row.kind, ":category": row.category, ":title": row.title,
            ":body": row.body, ":tags": row.tags, ":source": row.source, ":file_path": file_path,
            ":identity_key": row.identity_key, ":tier": row.tier, ":heat_tier": row.heat_tier,
            ":indexed": 1, ":created_at": row.created, ":updated_at": row.updated, ":expires_at": row.expires_at
        },
    )
}

fn update_row_by_file_path(tx: &Transaction, row: &FileRow, file_path: &str) -> rusqlite::Result<usize> {
    tx.execute(
        "UPDATE vault SET kind=:kind, category=:category, title=:title, body=:body, tags=:tags, \
         source=:source, identity_key=:identity_key, tier=:tier, heat_tier=:heat_tier, \
         updated_at=:updated_at, expires_at=:expires_at \
         WHERE file_path=:file_path",
        named_params! {
            ":kind": row.kind, ":category": row.category, ":title": row.title, ":body": row.body,
            ":tags": row.tags, ":source": row.source, ":identity_key": row.identity_key,
            ":tier": row.tier, ":heat_tier": row.heat_tier, ":updated_at": row.updated,
            ":expires_at": row.expires_at, ":file_path": file_path
        },
    )
}
