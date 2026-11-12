//! Data-segment checkpoints use decompressed offsets, stable across sealing.
use crate::{
    checkpoint::Checkpoint,
    update::{expired, Writer},
    Location, UpdateOptions, UpdateReport, VaultIndex,
};
use anyhow::{anyhow, bail, Result};
use cv_core::segment_read::SegmentFile;
use std::time::Instant;
use tantivy::Term;

pub(crate) fn run(
    index: &VaultIndex,
    options: &UpdateOptions,
    writer: &mut Writer,
    state: &mut Checkpoint,
    report: &mut UpdateReport,
    start: Instant,
) -> Result<()> {
    for segment in cv_core::segment_files(&index.root)? {
        if expired(options, start) {
            report.budget_reached = true;
            break;
        }
        if process(index, options, writer, state, report, &segment, start)? {
            break;
        }
    }
    Ok(())
}
fn process(
    index: &VaultIndex,
    options: &UpdateOptions,
    writer: &mut Writer,
    state: &mut Checkpoint,
    report: &mut UpdateReport,
    segment: &SegmentFile,
    start: Instant,
) -> Result<bool> {
    let key = format!("{}/{}/{}", segment.stream, segment.day, segment.part);
    if state.sealed.contains(&key) {
        return Ok(false);
    }
    let from = *state.segments.get(&key).unwrap_or(&0);
    validate_length(segment, from)?;
    let (offset, stopped) = read_part(index, options, writer, report, segment, from, start)?;
    if segment.sealed && !stopped {
        state.sealed.insert(key.clone());
    }
    state.segments.insert(key, offset);
    Ok(stopped)
}
fn validate_length(segment: &SegmentFile, from: u64) -> Result<()> {
    if !segment.sealed && std::fs::metadata(&segment.path)?.len() < from {
        bail!(
            "segment truncated after checkpoint: {}",
            segment.path.display()
        );
    }
    Ok(())
}
fn read_part(
    index: &VaultIndex,
    options: &UpdateOptions,
    writer: &mut Writer,
    report: &mut UpdateReport,
    segment: &SegmentFile,
    from: u64,
    start: Instant,
) -> Result<(u64, bool)> {
    let (mut offset, mut stopped) = (from, false);
    let result = cv_core::read_segment(segment, from, |loc, record| {
        if expired(options, start) {
            stopped = true;
            return Err(anyhow!("update budget reached"));
        }
        writer.delete_term(Term::from_field_text(index.fields.id, &record.id));
        offset = loc.offset + loc.len;
        writer.add_document(index.document(
            &record,
            &segment.stream,
            &Location::Segment { loc },
        )?)?;
        report.indexed += 1;
        Ok(())
    });
    match result {
        Ok(bad) => report.invalid_lines += bad,
        Err(_) if stopped => report.budget_reached = true,
        Err(error) => return Err(error),
    }
    Ok((offset, stopped))
}
