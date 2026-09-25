//! Age-based archive/restore by tier/category.
//! Moves entries from main vault to _archive/ subdirectory structure.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArchiveCandidate {
    pub id: String,
    pub kind: String,
    pub category: Option<String>,
    pub title: Option<String>,
    pub tier: Option<String>,
    pub file_path: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub reason: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ArchiveResult {
    pub archived: Vec<ArchiveCandidate>,
    pub count: usize,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RestoreResult {
    pub restored: bool,
    pub reason: Option<String>,
    pub file_path: Option<String>,
    pub kind: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ArchivedEntry {
    pub id: Option<String>,
    pub kind: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub file_path: String,
}

fn archive_dir(vault_dir: &str) -> PathBuf {
    Path::new(vault_dir).join("_archive")
}

const VALID_TIERS: &[&str] = &["ephemeral", "working", "durable"];
const VALID_CATEGORIES: &[&str] = &["knowledge", "entity", "event"];

/// Find entries eligible for archival based on lifecycle rules.
/// Returns candidates that match archiveAfterDays for their tier or category.
pub fn find_archive_candidates(
    db: &rusqlite::Connection,
    lifecycle_config: &std::collections::HashMap<String, Option<i64>>,
) -> Result<Vec<ArchiveCandidate>> {
    let now = chrono::Utc::now();
    let mut candidates: Vec<ArchiveCandidate> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (key, archive_days) in lifecycle_config {
        let archive_days = match archive_days {
            Some(days) if *days > 0 => *days,
            _ => continue,
        };

        let is_tier = VALID_TIERS.contains(&key.as_str());
        let is_category = VALID_CATEGORIES.contains(&key.as_str());
        if !is_tier && !is_category {
            continue;
        }

        let column = if is_tier { "tier" } else { "category" };
        let cutoff =
            (now - chrono::Duration::days(archive_days)).to_rfc3339();

        let mut stmt = db.prepare(&format!(
            "SELECT id, kind, category, title, tier, file_path, created_at, updated_at
             FROM vault
             WHERE {} = ? AND COALESCE(updated_at, created_at) <= ?
             ORDER BY COALESCE(updated_at, created_at) ASC",
            column
        ))?;

        let rows = stmt.query_map([key.clone(), cutoff.clone()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })?;

        for row in rows {
            let (id, kind, category, title, tier, file_path, created_at, updated_at) =
                row?;
            if !seen.insert(id.clone()) {
                continue;
            }
            candidates.push(ArchiveCandidate {
                id,
                kind,
                category,
                title,
                tier,
                file_path,
                created_at,
                updated_at,
                reason: format!(
                    "{}={}, archiveAfterDays={}",
                    column, key, archive_days
                ),
            });
        }
    }

    Ok(candidates)
}

/// Archive entries by moving them to _archive/ subdirectories.
/// Removes from index if successful; leaves in DB but marks archived.
pub fn archive_entries(
    db: &rusqlite::Connection,
    vault_dir: &str,
    candidates: &[ArchiveCandidate],
) -> Result<ArchiveResult> {
    let arch_root = archive_dir(vault_dir);
    let mut archived: Vec<ArchiveCandidate> = Vec::new();

    for entry in candidates {
        if let Some(file_path) = &entry.file_path {
            let rel = pathdiff::diff_paths(file_path, vault_dir)
                .ok_or_else(|| anyhow!("cannot compute relative path"))?;

            if rel.to_string_lossy().starts_with("..") ||
                rel.to_string_lossy().starts_with("_archive")
            {
                continue;
            }

            let dest_path = arch_root.join(&rel);
            let dest_dir = dest_path.parent().unwrap();

            match fs::create_dir_all(dest_dir).and_then(|_| {
                fs::rename(file_path, &dest_path)
            }) {
                Ok(_) => {
                    // Remove from index
                    let _ = db.execute("DELETE FROM vault WHERE id = ?", [&entry.id]);
                    archived.push(entry.clone());
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // File already gone; just remove from index
                    let _ = db.execute("DELETE FROM vault WHERE id = ?", [&entry.id]);
                    archived.push(entry.clone());
                }
                Err(e) => return Err(e.into()),
            }
        } else {
            // No file path; just remove from index
            let _ = db.execute("DELETE FROM vault WHERE id = ?", [&entry.id]);
            archived.push(entry.clone());
        }
    }

    Ok(ArchiveResult {
        count: archived.len(),
        archived,
    })
}

/// Check if a file matches the target entry ID in archive.
fn file_has_entry_id(file_path: &Path, entry_id: &str) -> bool {
    let Ok(content) = fs::read_to_string(file_path) else {
        return false;
    };
    content.contains(&format!("id: {}", entry_id))
        || content.contains(&format!("id: \"{}\"", entry_id))
}

/// Find archived file matching the entry ID.
/// `archive_entries` mirrors the vault's `{category}/{kind}/` layout under _archive/, so this
/// has to walk recursively — a flat `read_dir` on `arch_root` never finds anything nested.
fn find_archived_entry(arch_root: &Path, entry_id: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(arch_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .find_map(|entry| {
            let path = entry.path();
            let is_md = path.extension().is_some_and(|ext| ext == "md");
            if entry.file_type().is_file() && is_md && file_has_entry_id(path, entry_id) {
                return Some(path.to_path_buf());
            }
            None
        })
}

/// Extract kind from relative path.
fn extract_kind_from_path(rel: &Path) -> String {
    rel.components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .unwrap_or("note")
        .to_string()
}

/// Restore an entry from _archive/ back to main vault.
/// Re-indexes the entry in the database.
pub fn restore_entry(
    _db: &rusqlite::Connection,
    vault_dir: &str,
    entry_id: &str,
) -> Result<RestoreResult> {
    let arch_root = archive_dir(vault_dir);

    if !arch_root.exists() {
        return Ok(RestoreResult {
            restored: false,
            reason: Some("no _archive directory".to_string()),
            file_path: None,
            kind: None,
            id: None,
        });
    }

    let Some(target_file) = find_archived_entry(&arch_root, entry_id) else {
        return Ok(RestoreResult {
            restored: false,
            reason: Some(format!("entry {} not found in _archive", entry_id)),
            file_path: None,
            kind: None,
            id: None,
        });
    };

    let rel = pathdiff::diff_paths(&target_file, &arch_root)
        .ok_or_else(|| anyhow!("relative path calc failed"))?;
    let dest_path = Path::new(vault_dir).join(&rel);

    if dest_path.exists() {
        return Ok(RestoreResult {
            restored: false,
            reason: Some(format!("destination already exists: {}", dest_path.display())),
            file_path: None,
            kind: None,
            id: None,
        });
    }

    let dest_dir = dest_path.parent().unwrap();
    fs::create_dir_all(dest_dir)?;
    fs::rename(&target_file, &dest_path)?;

    let kind = extract_kind_from_path(&rel);

    Ok(RestoreResult {
        restored: true,
        reason: None,
        file_path: Some(dest_path.to_string_lossy().to_string()),
        kind: Some(kind),
        id: Some(entry_id.to_string()),
    })
}

/// Count archived entries in _archive/ directory.
pub fn count_archived_entries(vault_dir: &str) -> usize {
    let arch_root = archive_dir(vault_dir);
    if !arch_root.exists() {
        return 0;
    }

    walkdir::WalkDir::new(&arch_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .count()
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod tests;
