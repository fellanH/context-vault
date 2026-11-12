//! Writer acquisition and atomic publication; format-specific traversal is separate.
use crate::{checkpoint::Checkpoint, VaultIndex};
use anyhow::{bail, Result};
use cv_core::VaultType;
use serde::Serialize;
use std::time::{Duration, Instant};
use tantivy::{IndexWriter, TantivyDocument};

pub(crate) type Writer = IndexWriter<TantivyDocument>;
pub struct UpdateOptions {
    pub full: bool,
    /// Soft traversal/indexing budget. Commit/merge can take additional time.
    pub budget: Option<Duration>,
    pub writer_heap_bytes: usize,
    /// Knowledge files just written or deleted by this process: indexed first, unbudgeted.
    pub paths: Vec<std::path::PathBuf>,
}
impl Default for UpdateOptions {
    fn default() -> Self {
        Self {
            full: false,
            budget: Some(Duration::from_millis(200)),
            writer_heap_bytes: 64 * 1024 * 1024,
            paths: Vec::new(),
        }
    }
}
#[derive(Default, Debug, Serialize)]
pub struct UpdateReport {
    pub indexed: u64,
    pub removed: u64,
    pub invalid_lines: u64,
    /// Markdown files with frontmatter that is not a record (no id, bad fields): skipped, not indexed.
    pub invalid_records: u64,
    /// The first few skipped files with the reason, so a rebuild names what it left out.
    pub invalid_examples: Vec<String>,
    pub visited: u64,
    /// Directories re-listed because they changed since the last cycle.
    pub dirs_relisted: u64,
    pub busy: bool,
    pub budget_reached: bool,
    pub elapsed_ms: u128,
    pub writer_heap_bytes: usize,
}
impl VaultIndex {
    pub fn update(&self, options: &UpdateOptions) -> Result<UpdateReport> {
        validate(options)?;
        let start = Instant::now();
        let mut report = UpdateReport {
            writer_heap_bytes: options.writer_heap_bytes,
            ..Default::default()
        };
        let Some(mut writer) = acquire_writer(self, options, &mut report)? else {
            return Ok(report);
        };
        // Read after obtaining the lock; another writer may have committed since open.
        let mut state = self.checkpoint()?;
        let previous_payload = serde_json::to_string(&state)?;
        reset(&mut state, &mut writer, options.full)?;
        match self.kind {
            VaultType::Knowledge => {
                crate::knowledge_update::run(self, options, &mut writer, &mut state, &mut report)?
            }
            VaultType::Data => {
                crate::data_update::run(self, options, &mut writer, &mut state, &mut report, start)?
            }
        }
        publish(&mut writer, &state, &previous_payload, options.full)?;
        report.elapsed_ms = start.elapsed().as_millis();
        Ok(report)
    }
}
fn validate(options: &UpdateOptions) -> Result<()> {
    if options.writer_heap_bytes < 15_000_000 {
        bail!("writer heap must be at least 15 MB");
    }
    if options.full && options.budget.is_some() {
        bail!("full rebuild requires an explicit unbounded update");
    }
    Ok(())
}
fn acquire_writer(
    index: &VaultIndex,
    options: &UpdateOptions,
    report: &mut UpdateReport,
) -> Result<Option<Writer>> {
    match index
        .index
        .writer_with_num_threads(1, options.writer_heap_bytes)
    {
        Ok(writer) => Ok(Some(writer)),
        Err(tantivy::TantivyError::LockFailure(_, _)) => {
            report.busy = true;
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}
fn reset(state: &mut Checkpoint, writer: &mut Writer, full: bool) -> Result<()> {
    if full {
        writer.delete_all_documents()?;
        state.files.clear();
        state.invalid.clear();
        state.dirs.clear();
        state.segments.clear();
        state.sealed.clear();
        state.scan_after = None;
    }
    Ok(())
}
fn publish(
    writer: &mut Writer,
    state: &Checkpoint,
    previous_payload: &str,
    full: bool,
) -> Result<()> {
    let payload = serde_json::to_string(state)?;
    if full || payload != previous_payload {
        let mut commit = writer.prepare_commit()?;
        commit.set_payload(&payload);
        commit.commit()?;
    }
    Ok(())
}
pub(crate) fn expired(options: &UpdateOptions, start: Instant) -> bool {
    options
        .budget
        .is_some_and(|budget| start.elapsed() >= budget)
}
