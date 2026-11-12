//! `context-vault move <source> <dest>` — plain, agent-directed file relocation.
//! No kind/taxonomy inference (see cv_vault::relocate and specs/vault-taxonomy.md's
//! "known limitation" section on why `schema apply` can't safely do this on a vault
//! with heavy meta.folder use). DB sync is a targeted per-row file_path update, not a
//! full reindex -- fast even at tens of thousands of moved files, since it never
//! re-reads or re-parses any markdown.

use anyhow::Result;
use context_vault_core::paths::resolve_vault_dir;

use crate::engine_cmd;

pub async fn run_move(source: String, dest: String, dry_run: bool) -> Result<()> {
    let vault_dir = resolve_vault_dir()?;
    let summary = cv_vault::plan_move(&vault_dir, &source, &dest, dry_run)?;
    let verb = if dry_run { "would move" } else { "moved" };

    for (from, to) in &summary.moved {
        println!("{verb}: {from} -> {to}");
    }
    for path in &summary.skipped {
        println!("skip (destination already exists): {path}");
    }
    println!("{} {verb}, {} skipped", summary.moved.len(), summary.skipped.len());

    if dry_run {
        println!("dry run: nothing was written. Re-run without --dry-run to apply.");
        return Ok(());
    }
    if summary.moved.is_empty() {
        return Ok(());
    }

    let synced = sync_moved_paths(&summary.moved).await?;
    println!("synced {synced} DB row(s) (file_path only -- kind/category/tier untouched)");
    Ok(())
}

/// Rewrite `file_path` for exactly the rows that moved -- keyed by the old path, one
/// UPDATE per pair inside a single transaction. Deliberately not a full reindex: this
/// never re-reads or re-parses a markdown file, so it stays fast regardless of how
/// many entries the vault holds overall.
async fn sync_moved_paths(moved: &[(String, String)]) -> Result<usize> {
    let engine = engine_cmd::build_sqlite().await?;
    let db = engine.raw_db();
    let mut conn = db.lock().unwrap();
    let tx = conn.transaction()?;
    let mut updated = 0usize;
    {
        let mut stmt = tx.prepare("UPDATE vault SET file_path = ?1 WHERE file_path = ?2")?;
        for (from, to) in moved {
            updated += stmt.execute(rusqlite::params![to, from])?;
        }
    }
    tx.commit()?;
    Ok(updated)
}
