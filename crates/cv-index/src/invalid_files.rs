//! Markdown files with frontmatter that is not a record: one must not stop the whole index.
use crate::{checkpoint::Checkpoint, UpdateReport};

const MAX_INVALID_EXAMPLES: usize = 20;

/// Count and name a skipped file, and remember it by signature so it is not re-read until it changes.
pub(crate) fn note(
    report: &mut UpdateReport,
    state: &mut Checkpoint,
    key: String,
    signature: String,
    error: &anyhow::Error,
) {
    report.invalid_records += 1;
    if report.invalid_examples.len() < MAX_INVALID_EXAMPLES {
        report.invalid_examples.push(format!("{key}: {error:#}"));
    }
    state.invalid.insert(key, signature);
}
