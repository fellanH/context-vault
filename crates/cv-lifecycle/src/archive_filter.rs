//! Archive by filter (OPS-406): move every entry matching kind, tier and created range to
//! `_archive/`, out of the index and so out of every search, with nothing lost. A dry run
//! counts first; an apply writes a manifest (`archive_manifest`) before anything moves and
//! checks every moved file against it after. Entries with no file are left in place:
//! archiving them would only delete a row.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::archive::{archive_entries, ArchiveCandidate};
use crate::archive_manifest::{hash_file, write_manifest, ManifestRow};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ArchiveFilter {
    pub kinds: Vec<String>,
    pub tiers: Vec<String>,
    /// Created before this date or time (exclusive), as stored: ISO 8601.
    pub before: Option<String>,
    /// Created at or after this date or time.
    pub after: Option<String>,
}

impl ArchiveFilter {
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty() && self.tiers.is_empty() && self.before.is_none() && self.after.is_none()
    }
}

#[derive(Debug, Default)]
pub struct FilterReport {
    pub matched: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub no_file: usize,
    pub moved: usize,
    pub verified: usize,
    pub manifest: Option<PathBuf>,
}

/// The WHERE clause and its arguments for a filter.
fn where_clause(f: &ArchiveFilter) -> (String, Vec<String>) {
    let mut sql = String::from(" WHERE 1=1");
    let mut args: Vec<String> = Vec::new();
    for (col, list) in [("kind", &f.kinds), ("tier", &f.tiers)] {
        if !list.is_empty() {
            sql.push_str(&format!(" AND {col} IN ({})", vec!["?"; list.len()].join(",")));
            args.extend(list.iter().cloned());
        }
    }
    for (op, value) in [("<", &f.before), (">=", &f.after)] {
        if let Some(v) = value {
            sql.push_str(&format!(" AND created_at {op} ?"));
            args.push(v.clone());
        }
    }
    (sql, args)
}

fn candidate_from_row(r: &rusqlite::Row) -> rusqlite::Result<ArchiveCandidate> {
    Ok(ArchiveCandidate {
        id: r.get(0)?, kind: r.get(1)?, category: r.get(2)?, title: r.get(3)?, tier: r.get(4)?,
        file_path: r.get(5)?, created_at: r.get(6)?, updated_at: r.get(7)?, reason: "filter".to_string(),
    })
}

/// Indexed entries matching the filter. An empty filter is refused: it would match the whole vault.
pub fn select(db: &rusqlite::Connection, f: &ArchiveFilter) -> Result<Vec<ArchiveCandidate>> {
    if f.is_empty() {
        bail!("name at least one of --kind, --tier, --before or --after; an empty filter would archive the whole vault");
    }
    let (clause, args) = where_clause(f);
    let sql = format!("SELECT id, kind, category, title, tier, file_path, created_at, updated_at FROM vault{clause}");
    let mut stmt = db.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), candidate_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The vault-relative path of a file this archive would move, or None (outside the vault, already archived).
fn movable_rel(vault_dir: &str, file_path: &str) -> Option<String> {
    let rel = pathdiff::diff_paths(file_path, vault_dir)?.to_string_lossy().to_string();
    (!rel.starts_with("..") && !rel.starts_with("_archive")).then_some(rel)
}

/// Splits the matches into those with a file to move (and their manifest rows) and counts the rest.
fn plan(vault_dir: &str, all: Vec<ArchiveCandidate>, report: &mut FilterReport) -> (Vec<ArchiveCandidate>, Vec<ManifestRow>) {
    let (mut keep, mut rows) = (Vec::new(), Vec::new());
    for c in all {
        *report.by_kind.entry(c.kind.clone()).or_default() += 1;
        let rel = c.file_path.as_deref().and_then(|p| movable_rel(vault_dir, p));
        let hashed = rel.and_then(|rel| hash_file(&Path::new(vault_dir).join(&rel)).ok().map(|h| (rel, h)));
        match hashed {
            Some((rel, (bytes, fnv64))) => {
                rows.push(ManifestRow { id: c.id.clone(), rel, bytes, fnv64 });
                keep.push(c);
            }
            None => report.no_file += 1,
        }
    }
    (keep, rows)
}

/// How many moved files match their manifest row byte for byte.
fn verify(vault_dir: &str, rows: &[ManifestRow]) -> usize {
    let root = Path::new(vault_dir).join("_archive");
    rows.iter().filter(|r| hash_file(&root.join(&r.rel)).is_ok_and(|(b, h)| b == r.bytes && h == r.fnv64)).count()
}

/// Dry run unless `apply`: count what matches; with `apply`, write the manifest, move, then verify.
pub fn archive_by_filter(db: &rusqlite::Connection, vault_dir: &str, f: &ArchiveFilter, apply: bool) -> Result<FilterReport> {
    let all = select(db, f)?;
    let mut report = FilterReport { matched: all.len(), ..Default::default() };
    let (movable, rows) = plan(vault_dir, all, &mut report);
    if !apply {
        return Ok(report);
    }
    report.manifest = Some(write_manifest(vault_dir, f, &rows)?);
    report.moved = archive_entries(db, vault_dir, &movable)?.count;
    report.verified = verify(vault_dir, &rows);
    Ok(report)
}
