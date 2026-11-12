//! Gate G3 checks on the data vault as written: counts per kind, every id once, and
//! sampled records whose text matches the source byte for byte.
use super::{chat, markdown, Collected, MigrateOptions, MigrateReport};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::BufRead;
use std::path::{Path, PathBuf};

pub(crate) enum Sample {
    Markdown { rel: PathBuf, id: String },
    Chat { rel: PathBuf, line: usize, id: String },
}

impl Sample {
    pub(crate) fn id(&self) -> &str {
        match self {
            Sample::Markdown { id, .. } | Sample::Chat { id, .. } => id,
        }
    }
    pub(crate) fn same_source(&self, other: &Sample) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
    /// The text the source holds today, converted the same way as during the migration.
    fn source_text(&self, main: &Path) -> Result<String> {
        match self {
            Sample::Markdown { rel, .. } => {
                Ok(markdown::read(&main.join(rel), rel, Utc::now(), &Default::default())?.record.text)
            }
            Sample::Chat { rel, line, .. } => {
                let file = std::io::BufReader::new(std::fs::File::open(main.join(rel))?);
                let raw = file.split(b'\n').nth(*line).ok_or_else(|| anyhow!("line gone"))??;
                let record = chat::message(&String::from_utf8(raw)?)?.ok_or_else(|| anyhow!("no longer a message"))?;
                Ok(record.text)
            }
        }
    }
}

#[derive(Default, Serialize)]
pub struct VerifyReport {
    pub found: BTreeMap<String, u64>,
    pub counts_match: bool,
    pub missing_ids: u64,
    pub repeated_ids: u64,
    pub torn_lines: u64,
    pub samples_checked: u64,
    pub samples_identical: u64,
    pub passed: bool,
}

pub(crate) fn check(options: &MigrateOptions, report: &MigrateReport, collected: &Collected) -> Result<VerifyReport> {
    let wanted: HashMap<&str, &Sample> = collected.samples.iter().map(|s| (s.id(), s)).collect();
    let mut seen: HashSet<String> = HashSet::with_capacity(collected.ids.len());
    let mut v = VerifyReport::default();
    for segment in cv_core::segment_files(&options.exhaust)? {
        let torn = cv_core::read_segment(&segment, 0, |_, record| {
            *v.found.entry(record.kind.clone()).or_default() += 1;
            if let Some(sample) = wanted.get(record.id.as_str()) {
                v.samples_checked += 1;
                v.samples_identical += u64::from(sample.source_text(&options.main)? == record.text);
            }
            if !seen.insert(record.id) {
                v.repeated_ids += 1;
            }
            Ok(())
        })?;
        v.torn_lines += torn;
    }
    v.missing_ids = collected.ids.iter().filter(|id| !seen.contains(*id)).count() as u64;
    v.counts_match = v.found == report.converted;
    v.passed = v.counts_match
        && v.missing_ids == 0
        && v.repeated_ids == 0
        && v.torn_lines == 0
        && v.samples_checked == collected.samples.len() as u64
        && v.samples_checked > 0
        && v.samples_identical == v.samples_checked;
    Ok(v)
}
