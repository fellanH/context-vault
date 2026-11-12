//! Writing data-vault segments: append one record per line, and seal past days to zstd.
//!
//! Layout: `<vault>/segments/<stream>/<YYYY-MM>/<YYYY-MM-DD>.<part>.jsonl` while open,
//! `…/<YYYY-MM-DD>.<part>.jsonl.zst` once sealed. The part number never changes on
//! sealing and the decompressed bytes are identical, so a `Loc` (stream, day, part,
//! offset) stays valid across a seal: the index never has to be rewritten for it.
//!
//! Appends and seals take an exclusive lock on the open file, so several processes can
//! append to the same stream safely and an append never lands in a file being sealed.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use chrono::NaiveDate;

use crate::segment_read::{day_dir, segment_files, Loc};

/// One attempt: `None` when the part was sealed under us and the caller should retry.
pub(crate) fn try_append(vault_dir: &Path, stream: &str, day: NaiveDate, line: &str) -> Result<Option<Loc>> {
    let part = open_part(vault_dir, stream, day)?;
    let path = open_path(vault_dir, stream, day, part);
    std::fs::create_dir_all(path.parent().expect("segment path has a parent"))?;
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.lock()?;
    if sealed_under_us(&path, &file)? {
        return Ok(None); // the lock is released on drop
    }
    let offset = file.metadata()?.len();
    file.write_all(line.as_bytes())?;
    file.sync_data()?;
    file.unlock()?;
    Ok(Some(Loc { stream: stream.to_string(), day, part, offset, len: line.len() as u64 }))
}

/// True if this part was sealed between choosing it and locking it: either our handle
/// points at a removed file, or our open re-created a part that now also exists sealed.
/// In the second case the empty stray is removed so one Loc never names two files.
fn sealed_under_us(path: &Path, file: &File) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    if !path.with_extension("jsonl.zst").exists() {
        return Ok(false);
    }
    if file.metadata()?.len() == 0 {
        std::fs::remove_file(path)?;
    }
    Ok(true)
}

/// Seal every open segment whose day is before `today`: compress to `.jsonl.zst`
/// (written to a temp file, fsynced, renamed), then remove the open file. Returns the sealed paths.
pub fn seal_before(vault_dir: &Path, today: NaiveDate) -> Result<Vec<PathBuf>> {
    segment_files(vault_dir)?
        .into_iter()
        .filter(|seg| !seg.sealed && seg.day < today)
        .map(|seg| seal_one(&seg.path))
        .collect()
}

/// Compress one open part under its lock: temp file, fsync, rename, then remove the original.
fn seal_one(path: &Path) -> Result<PathBuf> {
    let file = File::open(path)?;
    file.lock()?;
    let target = path.with_extension("jsonl.zst");
    let tmp = path.with_extension("jsonl.zst.tmp");
    let mut encoder = zstd::Encoder::new(File::create(&tmp)?, 3)?;
    std::io::copy(&mut &file, &mut encoder)?;
    encoder.finish()?.sync_all()?;
    std::fs::rename(&tmp, &target)?;
    std::fs::remove_file(path)?;
    Ok(target)
}

/// The part to append to: the open (unsealed) part for the day if there is one, else
/// one past the highest sealed part.
fn open_part(vault_dir: &Path, stream: &str, day: NaiveDate) -> Result<u32> {
    let parts: Vec<(u32, bool)> = segment_files(vault_dir)?
        .into_iter()
        .filter(|s| s.stream == stream && s.day == day)
        .map(|s| (s.part, s.sealed))
        .collect();
    if let Some((p, _)) = parts.iter().find(|(_, sealed)| !sealed) {
        return Ok(*p);
    }
    Ok(parts.iter().map(|(p, _)| p + 1).max().unwrap_or(0))
}

fn open_path(vault_dir: &Path, stream: &str, day: NaiveDate, part: u32) -> PathBuf {
    day_dir(vault_dir, stream, day).join(format!("{}.{part}.jsonl", day.format("%Y-%m-%d")))
}

/// Streams become directory names: no separators, no dot-names, nothing empty.
pub(crate) fn check_stream(stream: &str) -> Result<()> {
    let ok = !stream.is_empty()
        && !stream.starts_with('.')
        && stream.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        bail!("stream name `{stream}`: use letters, digits, `-` or `_`");
    }
    Ok(())
}
