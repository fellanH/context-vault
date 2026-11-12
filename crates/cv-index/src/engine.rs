//! v5 behind the v4 engine trait (card 38): the MCP tools and CLI verbs run unchanged on
//! the vaults in `vaults.toml`, files as the source and one tantivy index per vault. Used
//! only when that registry lists vaults (written by `migrate`); otherwise v4 runs as before.
//! This file is the trait facade; opening, search and situations live beside it.
#[path = "engine_map.rs"]
mod map;
#[path = "engine_open.rs"]
mod open;
#[path = "engine_plan.rs"]
mod plan;
#[path = "engine_search.rs"]
mod search;
#[path = "engine_situations.rs"]
mod situations;
#[path = "engine_write.rs"]
mod write;

use crate::{UpdateOptions, VaultIndex};
use anyhow::Result;
use async_trait::async_trait;
use cv_core::VaultEntry;
use cv_engine::*;
use cv_vault::VaultTaxonomy;
use std::time::Instant;

struct Vault {
    name: String,
    index_dir: std::path::PathBuf,
    entry: VaultEntry,
    index: VaultIndex,
    taxonomy: VaultTaxonomy,
}

/// Where the engine keeps (and `migrate` builds) a vault's index.
pub fn index_dir(index_root: &std::path::Path, name: &str) -> std::path::PathBuf {
    index_root.join(name).join(open::INDEX_LAYOUT)
}

pub struct V5Backend {
    vaults: Vec<Vault>,
    /// Where saves go: the registry's default (or `$CONTEXT_VAULT`).
    default: usize,
}

// async_trait adds must_use to its boxed Future, which is already must_use.
#[allow(clippy::double_must_use)]
#[async_trait]
impl VaultEngine for V5Backend {
    async fn save(&self, input: SaveInput) -> Result<SaveOutput> {
        write::save(self, input)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        write::delete(self, id)
    }

    async fn get(&self, id: &str) -> Result<Option<Entry>> {
        Ok(self.find(id)?.map(|(_, entry)| entry))
    }

    async fn list(&self, opts: ListOptions) -> Result<Vec<Entry>> {
        search::list(self, &opts)
    }

    async fn hybrid_search(&self, query: &str, opts: SearchOptions) -> Result<Vec<ScoredEntry>> {
        search::run(self, query, &opts)
    }

    async fn fts_search(&self, query: &str, opts: SearchOptions) -> Result<Vec<ScoredEntry>> {
        search::run(self, query, &opts)
    }

    /// v5 has no embeddings (D1): BM25 only.
    async fn vec_search(&self, _embedding: &[f32], _opts: SearchOptions) -> Result<Vec<ScoredEntry>> {
        Ok(Vec::new())
    }

    /// No work at startup (D5).
    async fn init(&self) -> Result<()> {
        Ok(())
    }

    async fn reindex(&self, opts: ReindexOptions) -> Result<ReindexSummary> {
        let started = Instant::now();
        let mut summary = ReindexSummary { entries_reindexed: 0, duration_ms: 0.0, files_seen: 0, rows_removed: 0, prune_refused: None };
        for vault in &self.vaults {
            let (bytes, began) = (if opts.full { vault.index.plan()?.bytes } else { 0 }, Instant::now());
            let report = vault.index.update(&UpdateOptions { full: opts.full, budget: None, ..Default::default() })?;
            if opts.full {
                plan::record_rate(vault, bytes, began.elapsed());
            }
            summary.entries_reindexed += report.indexed;
            summary.files_seen += report.visited;
            summary.rows_removed += report.removed;
        }
        summary.duration_ms = started.elapsed().as_secs_f64() * 1000.0;
        Ok(summary)
    }

    async fn stats(&self) -> Result<VaultStats> {
        let mut stats = VaultStats { total_entries: 0, indexed_entries: 0, by_kind: Default::default(), by_tier: Default::default() };
        for vault in &self.vaults {
            self.catch_up(vault)?;
            stats.total_entries += vault.index.count()?;
            for (kind, n) in vault.index.kind_counts()? {
                *stats.by_kind.entry(kind.clone()).or_default() += n;
                *stats.by_tier.entry(vault.taxonomy.tier_for(&kind).to_string()).or_default() += n;
            }
        }
        stats.indexed_entries = stats.total_entries;
        Ok(stats)
    }

    async fn health(&self) -> Result<HealthStatus> {
        let names: Vec<String> = self.vaults.iter().map(|v| format!("{} ({:?})", v.name, v.entry.kind).to_lowercase()).collect();
        Ok(HealthStatus { ok: true, message: format!("v5 engine; vaults: {}", names.join(", ")) })
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
