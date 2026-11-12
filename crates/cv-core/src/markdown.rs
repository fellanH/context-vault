//! Knowledge-vault format: one markdown file per record, exactly as v4 writes it, so
//! existing vaults need no conversion. Writes go through `cv_vault::write_entry`, which
//! also runs the vault.toml refusers (typed fields, closed kinds, personal-free).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;

use cv_engine::SaveInput;
use cv_vault::{extract_custom_meta, parse_frontmatter};

use crate::record::Record;

/// Save `rec` as a markdown file. v4's writer mints the id and `created`; the returned
/// record carries the values actually written, and the file path.
pub fn write_record(vault_dir: &Path, rec: &Record) -> Result<(Record, PathBuf)> {
    let input = SaveInput {
        kind: rec.kind.clone(),
        title: rec.title.clone(),
        body: rec.text.clone(),
        tags: (!rec.tags.is_empty()).then(|| rec.tags.clone()),
        meta: (!rec.meta.is_empty()).then(|| Value::Object(rec.meta.clone())),
        source: rec.source.clone(),
        supersedes: (!rec.supersedes.is_empty()).then(|| rec.supersedes.clone()),
        ..Default::default()
    };
    let dir = vault_dir.to_str().ok_or_else(|| anyhow!("vault dir is not UTF-8"))?;
    let written = cv_vault::write_entry(dir, &input)?;
    let path = PathBuf::from(&written.file_path);
    Ok((read_record(&path)?, path))
}

/// Parse a markdown file into a record. `kind` falls back to the parent folder name, as in v4's reindex.
pub fn read_record(path: &Path) -> Result<Record> {
    read_record_or_mint(path, || None)
}

/// As `read_record`, but a file without an id takes `mint()`'s id. The file is not changed.
pub fn read_record_or_mint(path: &Path, mint: impl FnOnce() -> Option<String>) -> Result<Record> {
    let raw = std::fs::read_to_string(path)?;
    let (fm, body) = parse_frontmatter(&raw);
    let get = |k: &str| fm.get(k).and_then(|v| v.as_str()).map(str::to_string);
    let kind = get("kind")
        .or_else(|| path.parent()?.file_name()?.to_str().map(str::to_string))
        .unwrap_or_else(|| "note".to_string());
    let created = get("created")
        .and_then(|c| DateTime::parse_from_rfc3339(&c).ok())
        .map(|c| c.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let strings = |k: &str| -> Vec<String> {
        fm.get(k).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|t| t.as_str().map(str::to_string)).collect()).unwrap_or_default()
    };
    Ok(Record {
        id: get("id").or_else(mint).ok_or_else(|| anyhow!("{}: no id in frontmatter", path.display()))?,
        kind,
        created,
        text: body,
        title: get("title"),
        source: get("source"),
        tags: strings("tags"),
        meta: extract_custom_meta(&fm).unwrap_or_default(),
        supersedes: strings("supersedes"),
    })
}
