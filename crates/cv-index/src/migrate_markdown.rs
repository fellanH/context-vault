//! Exhaust markdown files (by folder, see `is_exhaust`) → data-vault records, one per file.
use super::{is_exhaust, sink::{stream_for, Sink}, stable_id, verify::Sample, Collected, MigrateOptions, MigrateReport};
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use cv_core::Record;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MAX_NAMED_SKIPS: usize = 50;

pub(crate) fn convert(options: &MigrateOptions, report: &mut MigrateReport, collected: &mut Collected) -> Result<()> {
    let started = Utc::now();
    let mut sink = Sink::new(&options.exhaust);
    for entry in walkdir::WalkDir::new(&options.main).sort_by_file_name() {
        let entry = entry?;
        let Some(rel) = exhaust_markdown(&options.main, &entry) else { continue };
        match read(entry.path(), &rel, started, &options.legacy_ids) {
            Ok(read) => keep(read, &rel, report, collected, &mut sink)?,
            Err(error) => skip(report, &rel, &format!("{error:#}")),
        }
    }
    sink.finish()
}

fn exhaust_markdown(root: &Path, entry: &walkdir::DirEntry) -> Option<PathBuf> {
    let is_md = entry.file_type().is_file() && entry.path().extension().is_some_and(|e| e == "md");
    let rel = entry.path().strip_prefix(root).ok()?;
    (is_md && is_exhaust(rel)).then(|| rel.to_path_buf())
}

pub(crate) struct Read {
    pub record: Record,
    pub id_from: IdFrom,
    pub dated_by_mtime: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdFrom {
    File,
    Legacy,
    Path,
}

/// One file as a record. A missing id comes from v4's database, else from the path; a
/// missing `created` (read as "now") becomes the file's modification time.
pub(crate) fn read(path: &Path, rel: &Path, started: DateTime<Utc>, legacy: &HashMap<String, String>) -> Result<Read> {
    let raw = std::fs::read_to_string(path)?;
    if !(raw.starts_with("---\n") || raw.starts_with("---\r\n")) {
        bail!("no frontmatter");
    }
    let mut id_from = IdFrom::File;
    let mut record = cv_core::markdown::read_record_or_mint(path, || {
        let key = rel.to_string_lossy();
        let (from, id) = match legacy.get(key.as_ref()) {
            Some(id) => (IdFrom::Legacy, id.clone()),
            None => (IdFrom::Path, stable_id(&format!("migrate:{key}"))),
        };
        id_from = from;
        Some(id)
    })?;
    let dated_by_mtime = record.created >= started;
    if dated_by_mtime {
        record.created = std::fs::metadata(path)?.modified()?.into();
    }
    Ok(Read { record, id_from, dated_by_mtime })
}

fn keep(read: Read, rel: &Path, report: &mut MigrateReport, collected: &mut Collected, sink: &mut Sink) -> Result<()> {
    let record = read.record;
    if !collected.ids.insert(record.id.clone()) {
        report.duplicate_ids += 1;
        skip(report, rel, &format!("duplicate id {}", record.id));
        return Ok(());
    }
    report.legacy_ids_used += u64::from(read.id_from == IdFrom::Legacy);
    report.minted_ids += u64::from(read.id_from == IdFrom::Path);
    report.dated_by_mtime += u64::from(read.dated_by_mtime);
    *report.converted.entry(record.kind.clone()).or_default() += 1;
    collected.sample(Sample::Markdown { rel: rel.to_path_buf(), id: record.id.clone() });
    sink.push(stream_for(&record.kind), record)
}

fn skip(report: &mut MigrateReport, rel: &Path, reason: &str) {
    report.skipped_count += 1;
    if report.skipped.len() < MAX_NAMED_SKIPS {
        report.skipped.push(format!("{}: {reason}", rel.display()));
    }
}
