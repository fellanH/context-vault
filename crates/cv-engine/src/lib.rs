use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod search;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub category: String,
    pub title: Option<String>,
    pub body: String,
    pub tags: Vec<String>,
    pub meta: Option<Value>,
    pub source: Option<String>,
    pub identity_key: Option<String>,
    pub tier: String,
    pub indexed: bool,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: Option<String>,
    pub superseded_by: Option<String>,
    pub supersedes: Option<Vec<String>>,
    pub related_to: Option<Vec<String>>,
    pub hit_count: i64,
    pub recall_count: i64,
    pub recall_sessions: i64,
    pub last_recalled_at: Option<String>,
    pub heat_tier: Option<String>,
    pub summary_condensed: Option<String>,
    pub summary_keypoint: Option<String>,
    /// Absolute path to the backing markdown file. NULL until the vault layer
    /// (Phase 1) writes the `.md`; the DB is an index over those files.
    pub file_path: Option<String>,
    pub last_accessed_at: Option<String>,
    /// JSON array of `{path, hash}` provenance records (stored as TEXT).
    pub source_files: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SaveInput {
    pub id: Option<String>,
    pub kind: String,
    pub title: Option<String>,
    pub body: String,
    pub tags: Option<Vec<String>>,
    pub meta: Option<Value>,
    pub source: Option<String>,
    pub identity_key: Option<String>,
    pub tier: Option<String>,
    pub expires_at: Option<String>,
    pub supersedes: Option<Vec<String>>,
    pub related_to: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveOutput {
    pub id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchOptions {
    pub kind: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tier: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    /// Project scope: entries tagged with a project-identity tag (`bucket:X` or
    /// `seat:X`) only match when X equals this value; entries carrying no
    /// project-identity tags are considered global and always match. `None`
    /// disables scoping entirely.
    pub scope_project: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredEntry {
    pub entry: Entry,
    pub score: f32,
    pub fts_rank: Option<f32>,
    pub vec_rank: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListOptions {
    pub kind: Option<String>,
    pub category: Option<String>,
    pub tier: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexOptions {
    /// Re-parse every file. Default (false) re-parses only files changed since the
    /// last successful reindex started.
    pub full: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexSummary {
    pub entries_reindexed: u64,
    pub duration_ms: f64,
    /// Markdown files found by the walk (reindexed or skipped as unchanged).
    #[serde(default)]
    pub files_seen: u64,
    /// Rows removed because their file is gone.
    #[serde(default)]
    pub rows_removed: u64,
    /// Set when the orphan sweep refused to run, with the reason.
    #[serde(default)]
    pub prune_refused: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStats {
    pub total_entries: u64,
    pub indexed_entries: u64,
    pub by_kind: std::collections::HashMap<String, u64>,
    pub by_tier: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub ok: bool,
    pub message: String,
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>>;
    fn dim(&self) -> usize;
}

#[async_trait]
pub trait VaultEngine: Send + Sync {
    async fn save(&self, input: SaveInput) -> anyhow::Result<SaveOutput>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<Entry>>;
    async fn list(&self, opts: ListOptions) -> anyhow::Result<Vec<Entry>>;
    async fn hybrid_search(
        &self,
        query: &str,
        opts: SearchOptions,
    ) -> anyhow::Result<Vec<ScoredEntry>>;
    async fn fts_search(
        &self,
        query: &str,
        opts: SearchOptions,
    ) -> anyhow::Result<Vec<ScoredEntry>>;
    async fn vec_search(
        &self,
        embedding: &[f32],
        opts: SearchOptions,
    ) -> anyhow::Result<Vec<ScoredEntry>>;
    async fn init(&self) -> anyhow::Result<()>;
    async fn reindex(&self, opts: ReindexOptions) -> anyhow::Result<ReindexSummary>;
    async fn stats(&self) -> anyhow::Result<VaultStats>;
    async fn health(&self) -> anyhow::Result<HealthStatus>;
}
