//! Buffers records per stream and writes them in batches (one write + fsync per day file).
use anyhow::Result;
use cv_core::Record;
use std::collections::BTreeMap;
use std::path::Path;

const BATCH: usize = 5_000;

pub(crate) struct Sink<'a> {
    dir: &'a Path,
    buffers: BTreeMap<String, Vec<Record>>,
}

impl<'a> Sink<'a> {
    pub(crate) fn new(dir: &'a Path) -> Self {
        Self { dir, buffers: BTreeMap::new() }
    }

    pub(crate) fn push(&mut self, stream: String, record: Record) -> Result<()> {
        let buffer = self.buffers.entry(stream.clone()).or_default();
        buffer.push(record);
        if buffer.len() >= BATCH {
            let batch = std::mem::take(buffer);
            cv_core::append_batch(self.dir, &stream, &batch)?;
        }
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<()> {
        for (stream, batch) in std::mem::take(&mut self.buffers) {
            if !batch.is_empty() {
                cv_core::append_batch(self.dir, &stream, &batch)?;
            }
        }
        Ok(())
    }
}

/// Kinds become stream directory names: lower-case letters, digits, `-` and `_` only.
pub(crate) fn stream_for(kind: &str) -> String {
    let stream: String = kind
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '-' })
        .collect();
    let stream = stream.trim_start_matches(['-', '.']).to_string();
    if stream.is_empty() { "unknown".into() } else { stream }
}
