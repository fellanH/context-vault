use cv_engine::{
    Embedder, Entry, ListOptions, SaveInput, SaveOutput, SearchOptions, ScoredEntry, VaultEngine,
};
use async_trait::async_trait;
use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::queries::entry_from_row;
use super::reindex;
use super::sqlite_ops;

const LIST_COLUMNS: &str = "SELECT id, kind, category, title, body, tags, meta, source, identity_key, tier, indexed, created_at, updated_at, expires_at, superseded_by, related_to, hit_count, recall_count, recall_sessions, last_recalled_at, heat_tier, summary_condensed, summary_keypoint, file_path, last_accessed_at, source_files FROM vault";

/// Build the `list` SQL from the filter options. Kept as a free function so the
/// trait method stays trivial (structural-contract complexity ceiling).
fn build_list_query(opts: &ListOptions) -> String {
    let mut where_clauses = vec![];
    if let Some(kind) = &opts.kind {
        where_clauses.push(format!("kind = '{}'", kind.replace('\'', "''")));
    }
    if let Some(category) = &opts.category {
        where_clauses.push(format!("category = '{}'", category.replace('\'', "''")));
    }
    if let Some(tier) = &opts.tier {
        where_clauses.push(format!("tier = '{}'", tier.replace('\'', "''")));
    }
    let mut query = LIST_COLUMNS.to_string();
    if !where_clauses.is_empty() {
        query.push_str(&format!(" WHERE {}", where_clauses.join(" AND ")));
    }
    query.push_str(&format!(
        " ORDER BY created_at DESC LIMIT {} OFFSET {}",
        opts.limit, opts.offset
    ));
    query
}

pub struct SqliteBackend {
    db: Arc<Mutex<Connection>>,
    vault_dir: PathBuf,
    embedder: Arc<dyn Embedder>,
}

impl SqliteBackend {
    pub fn new(db_path: &str, vault_dir: &str, embedder: Arc<dyn Embedder>) -> Result<Self> {
        sqlite_ops::register_sqlite_vec();
        let conn = Connection::open(db_path)?;
        // Each MCP client spawns its own `context-vault mcp` process (no shared daemon —
        // see stdio::run()), so independent OS processes hit this same db_path concurrently.
        // Without WAL + a busy_timeout, concurrent writers fail immediately with
        // "database is locked" instead of waiting (see project-management/wip/parallel-write-contention.md).
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;")?;
        Ok(SqliteBackend {
            db: Arc::new(Mutex::new(conn)),
            vault_dir: PathBuf::from(vault_dir),
            embedder,
        })
    }

    /// Raw connection handle for lifecycle operations (cv-lifecycle) that operate
    /// directly on SQL rather than through the VaultEngine trait.
    pub fn raw_db(&self) -> Arc<Mutex<Connection>> {
        self.db.clone()
    }

    pub fn vault_dir_path(&self) -> String {
        self.vault_dir.to_string_lossy().to_string()
    }
}

#[async_trait]
impl VaultEngine for SqliteBackend {
    async fn save(&self, input: SaveInput) -> Result<SaveOutput> {
        // Local-first: write the markdown file first (source of truth), then index it
        // into SQLite. On DB failure, roll the file back so disk and DB stay consistent.
        let vault_dir = self.vault_dir.to_string_lossy().to_string();
        let wr = cv_vault::write_entry(&vault_dir, &input)?;

        let text = match &input.title {
            Some(t) if !t.is_empty() => format!("{}\n{}", t, input.body),
            _ => input.body.clone(),
        };
        let embedding = self.embedder.embed(&text).await.ok();

        match sqlite_ops::save_entry(self.db.clone(), input, wr.clone(), embedding).await {
            Ok(out) => Ok(out),
            Err(e) => {
                // Best-effort rollback of the just-written file (mirrors TS captureAndIndex).
                let _ = std::fs::remove_file(&wr.file_path);
                Err(e)
            }
        }
    }

    async fn delete(&self, id: &str) -> Result<()> {
        sqlite_ops::delete_entry(self.db.clone(), id).await
    }

    async fn get(&self, id: &str) -> Result<Option<Entry>> {
        let id = id.to_string();
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || {
                let conn = db.lock().unwrap();
                super::queries::get_entry(&conn, &id)
            }
        })
        .await?
    }

    async fn list(&self, opts: ListOptions) -> Result<Vec<Entry>> {
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || {
                let conn = db.lock().unwrap();
                let query = build_list_query(&opts);
                let mut stmt = conn.prepare(&query)?;
                let entries = stmt
                    .query_map([], entry_from_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok::<_, anyhow::Error>(entries)
            }
        })
        .await?
    }

    async fn hybrid_search(
        &self,
        query: &str,
        opts: SearchOptions,
    ) -> Result<Vec<ScoredEntry>> {
        let fts_results = self.fts_search(query, opts.clone()).await?;
        // Embed the query for the vec lane. A signal-free embedding (NoOp) makes
        // vec_search return empty, so hybrid_search degrades cleanly to FTS-only.
        let query_embedding = self.embedder.embed(query).await.unwrap_or_default();
        let vec_results = self.vec_search(&query_embedding, opts).await?;
        Ok(super::queries::fuse_results(
            fts_results, vec_results,
        ))
    }

    async fn fts_search(
        &self,
        query: &str,
        opts: SearchOptions,
    ) -> Result<Vec<ScoredEntry>> {
        let query = query.to_string();
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || {
                let conn = db.lock().unwrap();
                super::queries::fts_search(&conn, &query, opts)
            }
        })
        .await?
    }

    async fn vec_search(
        &self,
        embedding: &[f32],
        opts: SearchOptions,
    ) -> Result<Vec<ScoredEntry>> {
        if sqlite_ops::is_zero_embedding(embedding) {
            return Ok(vec![]);
        }
        let embedding = embedding.to_vec();
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || {
                let conn = db.lock().unwrap();
                super::queries::vec_search(&conn, &embedding, opts)
            }
        })
        .await?
    }

    async fn init(&self) -> Result<()> {
        sqlite_ops::init_schema(self.db.clone(), self.embedder.dim()).await
    }

    async fn reindex(&self, opts: cv_engine::ReindexOptions) -> Result<cv_engine::ReindexSummary> {
        let vault_dir = self.vault_dir.to_string_lossy().to_string();
        reindex::reindex_entries(self.db.clone(), vault_dir, opts.full).await
    }

    async fn stats(&self) -> Result<cv_engine::VaultStats> {
        sqlite_ops::compute_stats(self.db.clone()).await
    }

    async fn health(&self) -> Result<cv_engine::HealthStatus> {
        Ok(sqlite_ops::get_health(self.db.clone()).await)
    }
}
