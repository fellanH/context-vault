//! Appending records to data-vault segments: one record, or many in one write per day
//! (a bulk migration would otherwise pay one segment scan and one fsync per record).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Result};
use chrono::NaiveDate;

use crate::record::Record;
use crate::segment_read::Loc;
use crate::segment_write::{check_stream, try_append};

/// Append `rec` to its day's open segment of `stream` (by `rec.created`, UTC). Returns where it landed.
pub fn append(vault_dir: &Path, stream: &str, rec: &Record) -> Result<Loc> {
    let mut locs = append_batch(vault_dir, stream, std::slice::from_ref(rec))?;
    Ok(locs.remove(0))
}

/// Append many records: all are checked before anything is written, then each day's
/// records go to its open part in one locked write and one fsync. Locations come back
/// in the order of `recs`.
pub fn append_batch(vault_dir: &Path, stream: &str, recs: &[Record]) -> Result<Vec<Loc>> {
    check_stream(stream)?;
    validate(vault_dir, recs)?;
    let mut by_day: BTreeMap<NaiveDate, Vec<(usize, String)>> = BTreeMap::new();
    for (i, rec) in recs.iter().enumerate() {
        let mut line = serde_json::to_string(rec)?;
        line.push('\n');
        by_day.entry(rec.created.date_naive()).or_default().push((i, line));
    }
    let mut locs: Vec<Option<Loc>> = vec![None; recs.len()];
    for (day, lines) in by_day {
        let chunk: String = lines.iter().map(|(_, line)| line.as_str()).collect();
        let at = append_chunk(vault_dir, stream, day, &chunk)?;
        let mut offset = at.offset;
        for (i, line) in lines {
            let len = line.len() as u64;
            locs[i] = Some(Loc { len, offset, ..at.clone() });
            offset += len;
        }
    }
    Ok(locs.into_iter().flatten().collect())
}

/// A seal can remove the file between our open and our lock; then the day has a new
/// part to append to. Three tries is plenty: sealing a given part happens once.
fn append_chunk(vault_dir: &Path, stream: &str, day: NaiveDate, chunk: &str) -> Result<Loc> {
    for _ in 0..3 {
        if let Some(loc) = try_append(vault_dir, stream, day, chunk)? {
            return Ok(loc);
        }
    }
    bail!("could not append to stream `{stream}` for {day}: segment kept being sealed")
}

// Refuse before creating directories or opening a segment.
fn validate(vault_dir: &Path, recs: &[Record]) -> Result<()> {
    let dir = vault_dir.to_str().ok_or_else(|| anyhow::anyhow!("vault dir is not UTF-8"))?;
    let taxonomy = cv_vault::taxonomy::VaultTaxonomy::load_strict(dir)?;
    for rec in recs {
        let input = cv_engine::SaveInput {
            kind: rec.kind.clone(), title: rec.title.clone(), body: rec.text.clone(),
            meta: Some(serde_json::Value::Object(rec.meta.clone())), tags: Some(rec.tags.clone()),
            ..Default::default()
        };
        cv_vault::schema_check::check_save(&taxonomy, &input)?;
    }
    Ok(())
}
