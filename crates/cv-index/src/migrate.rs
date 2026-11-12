//! v4 → v5 migration (spec: Migration from v4; card 39). Copies exhaust out of the
//! knowledge vault into a data vault and verifies the copy. It never changes or removes
//! a source file, except writing the id into knowledge files that have none (`knowledge`):
//! removing originals is a separate step that needs Felix's go.
#[path = "migrate_chat.rs"]
mod chat;
#[path = "migrate_knowledge.rs"]
mod knowledge;
#[path = "migrate_markdown.rs"]
mod markdown;
#[path = "migrate_setup.rs"]
pub mod setup;
#[path = "migrate_sink.rs"]
mod sink;
#[path = "migrate_verify.rs"]
mod verify;

use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub use verify::VerifyReport;

pub struct MigrateOptions {
    pub main: PathBuf,
    pub exhaust: PathBuf,
    /// Also convert `events/chat-logs/**/*.jsonl` transcripts to `chat-message` records.
    pub chat_logs: bool,
    /// v4's ids for files that carry none in their frontmatter, by path relative to `main`
    /// (read from the v4 database), so an entry keeps the id agents and links already use.
    pub legacy_ids: HashMap<String, String>,
}

#[derive(Default, Serialize)]
pub struct MigrateReport {
    /// Records written to the data vault, per kind.
    pub converted: BTreeMap<String, u64>,
    /// Exhaust files without an id that took their v4 database id (files unchanged).
    pub legacy_ids_used: u64,
    /// Exhaust files without an id and no v4 id either: given one derived from their path.
    pub minted_ids: u64,
    /// Exhaust files without a `created`: dated by their modification time.
    pub dated_by_mtime: u64,
    /// Knowledge files (staying in main) that had no id and now carry one in their frontmatter.
    pub knowledge_ids_written: u64,
    /// Of those, ids derived from the path because v4 had none either.
    pub knowledge_ids_minted: u64,
    /// The files that got an id (the first 200), relative to the vault.
    pub knowledge_id_files: Vec<String>,
    /// Exhaust files left behind, with the reason (no frontmatter, unreadable).
    pub skipped: Vec<String>,
    pub skipped_count: u64,
    /// Ids already written by an earlier file or message: not written twice.
    pub duplicate_ids: u64,
    pub chat: chat::ChatReport,
    pub sealed_segments: u64,
    pub verify: VerifyReport,
}

/// Where a file belongs: exhaust moves to the data vault, everything else stays in main.
/// By folder, not kind: `entities/task` holds real tasks that are edited in place, which a
/// data vault (append-only) can't do; `events/log/from-task` is exhaust (card 39, 2026-10-19).
pub fn is_exhaust(rel: &Path) -> bool {
    (rel.starts_with("events") && !rel.starts_with("events/chat-logs"))
        || rel.starts_with("knowledge/rule-candidate")
}

pub fn run(options: &MigrateOptions) -> Result<MigrateReport> {
    refuse_unless_fresh(options)?;
    let mut report = MigrateReport::default();
    let mut collected = Collected::default();
    knowledge::persist_ids(options, &mut report)?;
    markdown::convert(options, &mut report, &mut collected)?;
    if options.chat_logs {
        chat::convert(options, &mut report, &mut collected)?;
    }
    // History is complete: past days are sealed (zstd) as the data vault layout expects.
    report.sealed_segments = cv_core::seal_before(&options.exhaust, chrono::Utc::now().date_naive())?.len() as u64;
    report.verify = verify::check(options, &report, &collected)?;
    Ok(report)
}

/// Every id written (to refuse duplicates and to check each one arrived) and the samples to compare.
#[derive(Default)]
pub(crate) struct Collected {
    pub ids: HashSet<String>,
    pub samples: Vec<verify::Sample>,
}

/// Samples per source type: the first record, then about one in 4,096 by id, ten at most.
const SAMPLES_PER_SOURCE: usize = 10;

impl Collected {
    pub(crate) fn sample(&mut self, sample: verify::Sample) {
        let id = sample.id();
        let of_this_source = self.samples.iter().filter(|s| s.same_source(&sample)).count();
        // The first record of each source is always compared, so a small vault (where no id
        // hashes to ...000) still gets a byte-for-byte check.
        if (of_this_source == 0 || stable_id(id).ends_with("000")) && of_this_source < SAMPLES_PER_SOURCE {
            self.samples.push(sample);
        }
    }
}

/// A second run into the same data vault would duplicate every record.
fn refuse_unless_fresh(options: &MigrateOptions) -> Result<()> {
    if options.exhaust.starts_with(&options.main) || options.main.starts_with(&options.exhaust) {
        bail!("the data vault must not lie inside the knowledge vault (or the reverse)");
    }
    if options.exhaust.join("segments").exists() {
        bail!("{} already has segments; migrate into an empty data vault", options.exhaust.display());
    }
    Ok(())
}

/// A stable 128-bit FNV-1a hash, as 32 hex digits: ids minted from a path stay the same on every run.
pub(crate) fn stable_id(key: &str) -> String {
    let mut hash: u128 = 0x6c62272e07bb014262b821756295c58d;
    for byte in key.bytes() {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(0x0000000001000000000000000000013b);
    }
    format!("{hash:032x}")
}

#[cfg(test)]
#[path = "migrate_tests.rs"]
mod tests;
