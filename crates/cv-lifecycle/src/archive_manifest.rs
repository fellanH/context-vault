//! The manifest of an archive by filter (OPS-406): written under `_archive/manifests/` before
//! anything moves, one row per file (id, vault-relative path, bytes, FNV-1a 64), and the
//! restore that moves every file of it back.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::archive_filter::ArchiveFilter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestRow {
    pub id: String,
    pub rel: String,
    pub bytes: u64,
    pub fnv64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub created_at: String,
    pub filter: ArchiveFilter,
    pub rows: Vec<ManifestRow>,
}

/// FNV-1a 64: stable across builds and machines, enough to tell a moved file from a changed one.
pub fn fnv64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// A file's size and FNV-1a 64 as hex.
pub fn hash_file(path: &Path) -> Result<(u64, String)> {
    let bytes = fs::read(path)?;
    Ok((bytes.len() as u64, format!("{:016x}", fnv64(&bytes))))
}

pub fn write_manifest(vault_dir: &str, f: &ArchiveFilter, rows: &[ManifestRow]) -> Result<PathBuf> {
    let now = chrono::Utc::now();
    let dir = Path::new(vault_dir).join("_archive").join("manifests");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", now.format("%Y%m%dT%H%M%SZ")));
    if path.exists() {
        bail!("{} already exists; wait a second and run again", path.display());
    }
    let m = Manifest { created_at: now.to_rfc3339(), filter: f.clone(), rows: rows.to_vec() };
    fs::write(&path, serde_json::to_vec_pretty(&m)?)?;
    Ok(path)
}

enum Restore {
    Skipped,
    Moved { verified: bool },
}

fn restore_one(vault_dir: &str, r: &ManifestRow, apply: bool) -> Result<Restore> {
    let src = Path::new(vault_dir).join("_archive").join(&r.rel);
    let dst = Path::new(vault_dir).join(&r.rel);
    if dst.exists() || !src.exists() {
        return Ok(Restore::Skipped);
    }
    if !apply {
        return Ok(Restore::Moved { verified: false });
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&src, &dst)?;
    Ok(Restore::Moved { verified: hash_file(&dst).is_ok_and(|(b, h)| b == r.bytes && h == r.fnv64) })
}

/// Move every file of a manifest back from `_archive/` (dry run unless `apply`), never over an existing
/// file. Returns (restorable or restored, verified, skipped). The caller reindexes after an apply.
pub fn restore_by_manifest(vault_dir: &str, manifest: &Path, apply: bool) -> Result<(usize, usize, usize)> {
    let m: Manifest = serde_json::from_slice(&fs::read(manifest)?)?;
    let (mut moved, mut verified, mut skipped) = (0, 0, 0);
    for r in &m.rows {
        match restore_one(vault_dir, r, apply)? {
            Restore::Skipped => skipped += 1,
            Restore::Moved { verified: v } => {
                moved += 1;
                verified += usize::from(v);
            }
        }
    }
    Ok((moved, verified, skipped))
}
