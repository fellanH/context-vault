//! Reading data-vault segments: list them, stream records with their locations, and
//! fetch one record by location. Works the same on open (`.jsonl`) and sealed
//! (`.jsonl.zst`) parts, because offsets are into the decompressed bytes.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::record::Record;

/// Where a record lives in a data vault. Stable across sealing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loc {
    pub stream: String,
    pub day: NaiveDate,
    pub part: u32,
    /// Byte offset of the line in the (decompressed) segment.
    pub offset: u64,
    /// Line length in bytes, including the newline.
    pub len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentFile {
    pub stream: String,
    pub day: NaiveDate,
    pub part: u32,
    pub sealed: bool,
    pub path: PathBuf,
}

pub fn day_dir(vault_dir: &Path, stream: &str, day: NaiveDate) -> PathBuf {
    vault_dir.join("segments").join(stream).join(day.format("%Y-%m").to_string())
}

/// Every segment file in the vault, sorted by stream, day, part. Files that do not match
/// the naming scheme (temp files, strays) are ignored.
pub fn segment_files(vault_dir: &Path) -> Result<Vec<SegmentFile>> {
    let mut out = Vec::new();
    for stream in entries(&vault_dir.join("segments"), true) {
        let stream_name = stream.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        for month in entries(&stream, true) {
            out.extend(entries(&month, false).iter().filter_map(|f| parse_name(&stream_name, f)));
        }
    }
    out.sort_by(|a, b| (&a.stream, a.day, a.part).cmp(&(&b.stream, b.day, b.part)));
    Ok(out)
}

/// Entries of `dir` that are directories (`dirs = true`) or not. A missing dir is empty.
fn entries(dir: &Path, dirs: bool) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten().map(|e| e.path()).filter(|p| p.is_dir() == dirs).collect()
}

/// `2026-09-25.3.jsonl` or `2026-09-25.3.jsonl.zst`.
fn parse_name(stream: &str, path: &Path) -> Option<SegmentFile> {
    let name = path.file_name()?.to_str()?;
    let (stem, sealed) = match name.strip_suffix(".jsonl.zst") {
        Some(s) => (s, true),
        None => (name.strip_suffix(".jsonl")?, false),
    };
    let (day, part) = stem.split_once('.')?;
    Some(SegmentFile {
        stream: stream.to_string(),
        day: NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?,
        part: part.parse().ok()?,
        sealed,
        path: path.to_path_buf(),
    })
}

fn open_reader(seg: &SegmentFile) -> Result<Box<dyn BufRead>> {
    let file = File::open(&seg.path).with_context(|| format!("open {}", seg.path.display()))?;
    Ok(if seg.sealed {
        Box::new(BufReader::new(zstd::Decoder::new(file)?))
    } else {
        Box::new(BufReader::new(file))
    })
}

/// Call `f` for each record at or after byte `from` in `seg`. A line that does not parse
/// (e.g. torn by a crash mid-write) is skipped and counted; the count is returned so the
/// caller can report it rather than lose it silently.
pub fn read_segment(seg: &SegmentFile, from: u64, mut f: impl FnMut(Loc, Record) -> Result<()>) -> Result<u64> {
    let mut reader = open_reader(seg)?;
    std::io::copy(&mut reader.by_ref().take(from), &mut std::io::sink())?;
    let (mut offset, mut bad, mut line) = (from, 0u64, String::new());
    loop {
        line.clear();
        let n = reader.read_line(&mut line)? as u64;
        if n == 0 {
            return Ok(bad);
        }
        let loc = Loc { stream: seg.stream.clone(), day: seg.day, part: seg.part, offset, len: n };
        match parse_line(&line) {
            Some(rec) => f(loc, rec)?,
            None => bad += 1,
        }
        offset += n;
    }
}

/// A complete line (newline-terminated) holding one record; anything else is torn or corrupt.
fn parse_line(line: &str) -> Option<Record> {
    if !line.ends_with('\n') {
        return None;
    }
    serde_json::from_str(line.trim_end()).ok()
}

/// Fetch the record at `loc`, from whichever form (open or sealed) the part is in now.
pub fn read_at(vault_dir: &Path, loc: &Loc) -> Result<Record> {
    let dir = day_dir(vault_dir, &loc.stream, loc.day);
    let base = format!("{}.{}.jsonl", loc.day.format("%Y-%m-%d"), loc.part);
    let mut buf = vec![0u8; loc.len as usize];
    if dir.join(&base).exists() {
        let mut file = File::open(dir.join(&base))?;
        file.seek(SeekFrom::Start(loc.offset))?;
        file.read_exact(&mut buf)?;
    } else {
        let path = dir.join(format!("{base}.zst"));
        let mut dec = zstd::Decoder::new(File::open(&path).map_err(|e| anyhow!("{}: {e}", path.display()))?)?;
        std::io::copy(&mut (&mut dec).take(loc.offset), &mut std::io::sink())?;
        dec.read_exact(&mut buf)?;
    }
    Ok(serde_json::from_slice(&buf)?)
}
