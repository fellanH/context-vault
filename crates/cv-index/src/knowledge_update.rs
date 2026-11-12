//! Resumable knowledge-file scans and removal of source records that disappeared.
use crate::source_file::{has_frontmatter, is_markdown, signature};
use crate::{
    checkpoint::{Checkpoint, FileState},
    update::{expired, Writer},
    Location, UpdateOptions, UpdateReport, VaultIndex,
};
use anyhow::{bail, Result};
use cv_core::Record;
use std::{
    collections::HashMap,
    path::Path,
    time::{Duration, Instant},
};
use tantivy::Term;

#[path = "dirty_dirs.rs"]
mod dirty_dirs;
#[path = "written_files.rs"]
mod written_files;

pub(crate) fn run(
    index: &VaultIndex,
    options: &UpdateOptions,
    writer: &mut Writer,
    state: &mut Checkpoint,
    report: &mut UpdateReport,
) -> Result<()> {
    let owners = state
        .files
        .iter()
        .map(|(path, file)| (file.id.clone(), path.clone()))
        .collect();
    let resume_after = state.scan_after.clone();
    let skipping_prefix = resume_after
        .as_ref()
        .is_some_and(|path| index.root.join(path).is_file());
    let mut scan = Scan {
        index,
        options,
        writer,
        state,
        report,
        owners,
        resume_after,
        skipping_prefix,
        clock: None,
        dirty_spent: Duration::ZERO,
    };
    scan.run()
}
struct Scan<'a> {
    index: &'a VaultIndex,
    options: &'a UpdateOptions,
    writer: &'a mut Writer,
    state: &'a mut Checkpoint,
    report: &'a mut UpdateReport,
    owners: HashMap<String, String>,
    resume_after: Option<String>,
    skipping_prefix: bool,
    clock: Option<Instant>,
    /// Time the changed-directory pass used; the lap gets the rest of the budget.
    dirty_spent: Duration,
}
impl Scan<'_> {
    fn run(&mut self) -> Result<()> {
        self.refresh_written()?;
        self.dirty_spent = self.refresh_dirty_dirs()?;
        if self.report.budget_reached {
            return Ok(());
        }
        // Resuming must not re-walk everything before the cursor: on a large vault that
        // prefix alone outlasts the budget and the scan stops advancing (card 37).
        let resume = self
            .resume_after
            .as_ref()
            .filter(|_| self.skipping_prefix)
            .map(|key| self.index.root.join(key));
        let root = self.index.root.clone();
        let walker = walkdir::WalkDir::new(&self.index.root)
            .follow_links(false)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(move |entry| crate::scan_cursor::keep_walking(&root, entry.path(), resume.as_deref()));
        for entry in walker {
            if !self.visit(&entry?)? {
                break;
            }
        }
        if !self.report.budget_reached {
            self.prune();
        }
        Ok(())
    }
    fn visit(&mut self, entry: &walkdir::DirEntry) -> Result<bool> {
        if entry.file_type().is_dir() {
            self.note_dir(entry)?;
            return Ok(true);
        }
        if !is_markdown(entry) {
            return Ok(true);
        }
        self.report.visited += 1;
        let key = entry
            .path()
            .strip_prefix(&self.index.root)?
            .to_string_lossy()
            .into_owned();
        if self.skip_prefix(&key) {
            return Ok(true);
        }
        // The budget covers work past the cursor, not the fixed cost of reaching it: listing a
        // 45k-file directory alone can outlast 200 ms, and then no cycle would advance.
        let spent = self.dirty_spent;
        let start = *self.clock.get_or_insert_with(|| {
            Instant::now()
                .checked_sub(spent)
                .unwrap_or_else(Instant::now)
        });
        if self.stop_for_budget(start) {
            return Ok(false);
        }
        self.state.scan_after = Some(key.clone());
        self.process(entry.path(), &entry.metadata()?, key)?;
        Ok(true)
    }
    fn stop_for_budget(&mut self, start: Instant) -> bool {
        self.report.budget_reached = expired(self.options, start);
        self.report.budget_reached
    }
    fn skip_prefix(&mut self, key: &str) -> bool {
        if !self.skipping_prefix {
            return false;
        }
        if self.resume_after.as_deref() == Some(key) {
            self.skipping_prefix = false;
        }
        true
    }
    fn process(&mut self, path: &Path, metadata: &std::fs::Metadata, key: String) -> Result<()> {
        // Exhaust belongs to the data vault (migrate copies it there); the originals that stay
        // must not be searched twice or crowd default search.
        if crate::migrate::is_exhaust(Path::new(&key)) {
            self.forget(&key);
            return Ok(());
        }
        let signature = signature(metadata)?;
        if self
            .state
            .files
            .get(&key)
            .is_some_and(|old| old.signature == signature)
        {
            return Ok(());
        }
        if self.state.invalid.get(&key) == Some(&signature) {
            self.report.invalid_records += 1;
            return Ok(());
        }
        let raw = std::fs::read_to_string(path)?;
        if !has_frontmatter(&raw) {
            self.forget(&key);
            return Ok(());
        }
        let record = match cv_core::markdown::read_record(path) {
            Ok(record) => record,
            Err(error) => {
                self.forget(&key);
                crate::invalid_files::note(self.report, self.state, key, signature, &error);
                return Ok(());
            }
        };
        self.check_identity(&key, &record)?;
        self.state.invalid.remove(&key);
        self.replace(key, record, signature)
    }
    fn check_identity(&self, key: &str, record: &Record) -> Result<()> {
        if let Some(owner) = self.owners.get(&record.id) {
            if owner != key && self.index.root.join(owner).is_file() {
                bail!(
                    "duplicate id `{}` in {} and {}; repair identities before indexing",
                    record.id,
                    owner,
                    key
                );
            }
        }
        Ok(())
    }
    fn replace(&mut self, key: String, record: Record, signature: String) -> Result<()> {
        if let Some(old) = self.state.files.get(&key) {
            self.owners.remove(&old.id);
            self.writer
                .delete_term(Term::from_field_text(self.index.fields.id, &old.id));
        }
        self.owners.insert(record.id.clone(), key.clone());
        self.writer
            .delete_term(Term::from_field_text(self.index.fields.id, &record.id));
        self.writer.add_document(self.index.document(
            &record,
            "",
            &Location::Markdown {
                path: key.clone().into(),
            },
        )?)?;
        self.state.files.insert(
            key,
            FileState {
                signature,
                id: record.id,
            },
        );
        self.report.indexed += 1;
        Ok(())
    }
    fn forget(&mut self, key: &str) {
        if let Some(old) = self.state.files.remove(key) {
            self.owners.remove(&old.id);
            self.writer
                .delete_term(Term::from_field_text(self.index.fields.id, &old.id));
            self.report.removed += 1;
        }
    }
    fn prune(&mut self) {
        self.state.scan_after = None;
        let deleted: Vec<_> = self
            .state
            .files
            .keys()
            .filter(|key| !self.index.root.join(key).is_file())
            .cloned()
            .collect();
        for key in deleted {
            self.forget(&key);
        }
        let root = &self.index.root;
        self.state.invalid.retain(|key, _| root.join(key).is_file());
        self.state.dirs.retain(|key, _| root.join(key).is_dir());
    }
}
