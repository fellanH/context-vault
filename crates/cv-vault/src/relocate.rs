//! Plain, agent-directed file relocation: `move(source, dest)`, no kind/taxonomy
//! inference at all. `schema apply` infers destinations from a vault.toml taxonomy,
//! which can't tell deliberate structure (volume-isolation subfolders, archival
//! layouts) apart from genuine drift on a vault that uses `meta.folder` heavily. This
//! module is the escape hatch: an agent names the exact source and destination, gets
//! mv-like semantics, and nothing is ever overwritten.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

#[derive(Debug, Default)]
pub struct MoveSummary {
    pub source_root: String,
    pub dest_root: String,
    pub moved: Vec<(String, String)>,
    pub skipped: Vec<String>,
}

fn assert_relative_no_escape(label: &str, rel: &str) -> Result<()> {
    if rel.is_empty() || Path::new(rel).is_absolute() {
        return Err(anyhow!("{label} must be a relative path: {rel}"));
    }
    let escapes = Path::new(rel).components().any(|c| c.as_os_str() == "..");
    if escapes {
        return Err(anyhow!("{label} may not contain '..': {rel}"));
    }
    Ok(())
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(root)? {
        collect_one_entry(entry?.path(), &mut out)?;
    }
    Ok(out)
}

fn collect_one_entry(path: PathBuf, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_dir() {
        out.extend(collect_files(&path)?);
        return Ok(());
    }
    if path.is_file() {
        out.push(path);
    }
    Ok(())
}

fn plan_one_file(source_root: &Path, dest_root: &Path, file: &Path) -> Result<(PathBuf, PathBuf)> {
    let rel = file.strip_prefix(source_root).unwrap_or(file);
    let dest_file = if source_root.is_file() {
        dest_root.to_path_buf()
    } else {
        dest_root.join(rel)
    };
    Ok((file.to_path_buf(), dest_file))
}

/// Move `source` (relative to `vault_dir`) so its contents live at `dest` (relative
/// to `vault_dir`): a directory's contents land directly under `dest` (merge/flatten
/// semantics, not `mv`'s nest-under-basename -- the common vault-reorg need is
/// "make these live over there," not "file this folder inside that one"; ask for the
/// latter by spelling `dest` out, e.g. `dest = "archive/tool-calls"`). `dry_run: true`
/// computes the same plan without touching disk. A file is skipped -- never
/// overwritten -- when something already exists at its destination.
pub fn plan_move(vault_dir: &str, source: &str, dest: &str, dry_run: bool) -> Result<MoveSummary> {
    assert_relative_no_escape("source", source)?;
    assert_relative_no_escape("dest", dest)?;

    let source_root = Path::new(vault_dir).join(source);
    if !source_root.exists() {
        return Err(anyhow!("source does not exist: {source}"));
    }
    let dest_root = Path::new(vault_dir).join(dest);

    let mut summary = MoveSummary {
        source_root: source_root.to_string_lossy().to_string(),
        dest_root: dest_root.to_string_lossy().to_string(),
        ..Default::default()
    };

    for file in collect_files(&source_root)? {
        let (from, to) = plan_one_file(&source_root, &dest_root, &file)?;
        record_move(&mut summary, from, to, dry_run)?;
    }

    if !dry_run {
        remove_if_empty_dir(&source_root);
    }

    Ok(summary)
}

fn record_move(summary: &mut MoveSummary, from: PathBuf, to: PathBuf, dry_run: bool) -> Result<()> {
    if to.exists() {
        summary.skipped.push(from.to_string_lossy().to_string());
        return Ok(());
    }
    if !dry_run {
        execute_move(&from, &to)?;
    }
    summary
        .moved
        .push((from.to_string_lossy().to_string(), to.to_string_lossy().to_string()));
    Ok(())
}

fn execute_move(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(from, to)?;
    Ok(())
}

/// Best-effort cleanup: a whole-folder move leaves an empty directory tree behind;
/// prune it bottom-up so the vault doesn't accumulate empty husks. A directory that
/// still has anything left in it is untouched -- remove_dir only succeeds on empty.
fn remove_if_empty_dir(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for child in entries.flatten() {
        let path = child.path();
        if path.is_dir() {
            remove_if_empty_dir(&path);
        }
    }
    let _ = fs::remove_dir(dir);
}

#[cfg(test)]
#[path = "relocate_tests.rs"]
mod tests;
