//! Lifecycle commands: archive, restore, summarize, tier, assemble.
//!
//! `dupes`, `stale`, `compact`, `consolidate`, `prune` were cut entirely from the CLI —
//! their backing functions were themselves dry-run/empty stubs (cv_lifecycle::consolidation::
//! find_duplicates always returned an empty Vec; cv_lifecycle::compact::compact always
//! returned zero counts) or had no backend at all (prune, stale). See
//! project-management/decisions.md for the call to defer real implementation rather than
//! ship commands that "succeed" and do nothing.

use std::collections::HashMap;

use anyhow::Result;
use context_vault_core::engine::VaultEngine;
use cv_lifecycle::archive_filter::{archive_by_filter, ArchiveFilter};
use cv_lifecycle::archive_manifest::restore_by_manifest;
use cv_lifecycle::{
    archive_entries, find_archive_candidates, generate_summary_tiers, restore_entry,
    run_tier_analysis, TierAnalysisOptions,
};

use crate::engine_cmd;

pub async fn run_archive(days: Option<u32>, filter: ArchiveFilter, apply: bool) -> Result<()> {
    let engine = engine_cmd::build_sqlite().await?;
    if !filter.is_empty() {
        let db = engine.raw_db();
        let report = archive_by_filter(&db.lock().unwrap(), &engine.vault_dir_path(), &filter, apply)?;
        print_filter_report(&report, apply);
        return Ok(());
    }
    run_archive_age(&engine, days.unwrap_or(365) as i64, apply)
}

/// Age mode: entries untouched for more than `days` days (dry run unless `apply`).
fn run_archive_age(engine: &context_vault_core::backends::SqliteBackend, days: i64, apply: bool) -> Result<()> {

    let mut config: HashMap<String, Option<i64>> = HashMap::new();
    for tier in ["ephemeral", "working", "durable"] {
        config.insert(tier.to_string(), Some(days));
    }

    let db = engine.raw_db();
    let vault_dir = engine.vault_dir_path();
    let candidates;
    let result;
    {
        let conn = db.lock().unwrap();
        candidates = find_archive_candidates(&conn, &config)?;
        if !apply {
            println!("dry run: {} entries untouched for more than {days} days would be archived. Add --apply to archive them.", candidates.len());
            return Ok(());
        }
        result = archive_entries(&conn, &vault_dir, &candidates)?;
    }

    println!("archived {} entries (>{} days untouched)", result.count, days);
    for c in &result.archived {
        println!("  {} — {}", c.id, c.title.as_deref().unwrap_or("(untitled)"));
    }
    Ok(())
}

/// OPS-406: every file of an archive manifest back in place (dry run unless `apply`), then a reindex.
async fn run_restore_manifest(manifest: &std::path::Path, apply: bool) -> Result<()> {
    let engine = engine_cmd::build_sqlite().await?;
    let (n, verified, skipped) = restore_by_manifest(&engine.vault_dir_path(), manifest, apply)?;
    if !apply {
        println!("dry run: {n} entries would be restored, {skipped} skipped (already in place or missing in _archive). Add --apply.");
        return Ok(());
    }
    let summary = engine.reindex(context_vault_core::engine::ReindexOptions { full: false }).await?;
    println!("restored {n} ({verified} verified byte for byte), {skipped} skipped; reindexed {} entries.", summary.entries_reindexed);
    Ok(())
}

/// OPS-406: what a filter archive matched, moved and verified, or would move on a dry run.
fn print_filter_report(r: &cv_lifecycle::archive_filter::FilterReport, apply: bool) {
    for (kind, n) in &r.by_kind {
        println!("{n:>8}  {kind}");
    }
    println!("{} entries match; {} have no file in the vault and stay where they are.", r.matched, r.no_file);
    match &r.manifest {
        Some(m) if apply => println!("archived {}; {} of them verified byte for byte in _archive/. Manifest: {}. Undo: context-vault restore --manifest {} --apply", r.moved, r.verified, m.display(), m.display()),
        _ => println!("dry run: nothing moved. Add --apply to archive {} entries (a manifest is written first).", r.matched - r.no_file),
    }
}

pub async fn run_restore(id: Option<String>, manifest: Option<std::path::PathBuf>, apply: bool) -> Result<()> {
    if let Some(m) = manifest {
        return run_restore_manifest(&m, apply).await;
    }
    let engine = engine_cmd::build_sqlite().await?;
    let id = id.ok_or_else(|| anyhow::anyhow!("name an entry id, or --manifest <file>"))?;
    let id = id.as_str();
    let db = engine.raw_db();
    let vault_dir = engine.vault_dir_path();
    let result = {
        let conn = db.lock().unwrap();
        restore_entry(&conn, &vault_dir, id)?
    };

    if result.restored {
        println!("restored {} → {}", id, result.file_path.unwrap_or_default());
    } else {
        println!("not restored: {}", result.reason.unwrap_or_else(|| "unknown reason".to_string()));
    }
    Ok(())
}

fn fetch_unsummarized(conn: &rusqlite::Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, body FROM vault WHERE indexed != 0 AND (summary_condensed IS NULL OR summary_keypoint IS NULL)",
    )?;
    let rows = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn apply_summaries(conn: &rusqlite::Connection, rows: &[(String, String)]) -> Result<usize> {
    let mut updated = 0;
    for (id, body) in rows {
        let tiers = generate_summary_tiers(body);
        conn.execute(
            "UPDATE vault SET summary_condensed = ?, summary_keypoint = ? WHERE id = ?",
            rusqlite::params![tiers.condensed, tiers.keypoint, id],
        )?;
        updated += 1;
    }
    Ok(updated)
}

pub async fn run_summarize() -> Result<()> {
    let engine = engine_cmd::build_sqlite().await?;
    let db = engine.raw_db();

    let updated = {
        let conn = db.lock().unwrap();
        let rows = fetch_unsummarized(&conn)?;
        apply_summaries(&conn, &rows)?
    };

    println!("summarized {} entries", updated);
    Ok(())
}

pub async fn run_tier() -> Result<()> {
    let engine = engine_cmd::build_sqlite().await?;
    let db = engine.raw_db();
    let report = {
        let conn = db.lock().unwrap();
        run_tier_analysis(&conn, &TierAnalysisOptions::default())?
    };

    println!(
        "tier analysis: {} hot, {} cold, {} reset to neutral, {} bundles",
        report.hot_entries.len(),
        report.cold_entries.len(),
        report.warm_reset,
        report.bundles.len()
    );
    for e in &report.hot_entries {
        println!("  hot  {} — {} ({} accesses)", e.id, e.title.as_deref().unwrap_or("(untitled)"), e.access_count);
    }
    for e in &report.cold_entries {
        println!("  cold {} — {}", e.id, e.title.as_deref().unwrap_or("(untitled)"));
    }
    Ok(())
}

pub async fn run_assemble(ids: Vec<String>) -> Result<()> {
    let engine = engine_cmd::build_sqlite().await?;

    for id in &ids {
        if let Some(entry) = engine.get(id).await? {
            println!("# {}", entry.title.unwrap_or_default());
            println!("{}", entry.body);
            println!();
        }
    }
    Ok(())
}
