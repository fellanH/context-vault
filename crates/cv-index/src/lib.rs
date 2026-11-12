//! Disposable search indexes. Source records and commit checkpoints advance together.
//! Staged alongside v4: `engine::V5Backend` serves the MCP tools only when vaults.toml lists vaults.
mod checkpoint;
mod data_update;
pub mod engine;
#[path = "engine_entry.rs"]
mod engine_entry;
mod invalid_files;
mod knowledge_update;
mod lookup;
pub mod plan;
#[path = "lookup_situations.rs"]
mod lookup_situations;
pub mod migrate;
mod scan_cursor;
mod search;
mod source_file;
mod update;
pub use update::{UpdateOptions, UpdateReport};

use anyhow::{bail, Result};
use cv_core::{Loc, Record, VaultType};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tantivy::schema::{Field, Schema, FAST, STORED, STRING, TEXT};
use tantivy::{Index, TantivyDocument};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "lowercase")]
pub enum Location {
    Markdown { path: PathBuf },
    Segment { loc: Loc },
}

#[derive(Debug, Serialize)]
pub struct Hit {
    pub score: f32,
    pub record: Record,
}

#[derive(Default)]
pub struct SearchOptions {
    pub kind: Option<String>,
    pub stream: Option<String>,
    pub tag: Option<String>,
    /// Exact typed scalar matches; implemented with indexed canonical key/value terms.
    pub meta: serde_json::Map<String, serde_json::Value>,
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: usize,
}

pub struct VaultIndex {
    index: Index,
    root: PathBuf,
    kind: VaultType,
    fields: Fields,
}

struct Fields {
    id: Field,
    title: Field,
    text: Field,
    kind: Field,
    stream: Field,
    tags: Field,
    meta: Field,
    created: Field,
    loc: Field,
    /// Ids this record replaces, so "is X superseded?" is one term lookup.
    supersedes: Field,
    /// `meta.situations` names, for the situation reads (card 38).
    situations: Field,
}

impl Fields {
    fn schema() -> Schema {
        let mut s = Schema::builder();
        s.add_text_field("id", STRING | STORED);
        s.add_text_field("title", TEXT);
        s.add_text_field("text", TEXT);
        s.add_text_field("kind", STRING | FAST);
        s.add_text_field("stream", STRING | FAST);
        s.add_text_field("tags", STRING);
        s.add_text_field("meta", STRING);
        s.add_date_field("created", FAST);
        s.add_text_field("loc", STORED);
        s.add_text_field("supersedes", STRING);
        s.add_text_field("situations", STRING);
        s.build()
    }
    fn from_schema(s: &Schema) -> Result<Self> {
        const NAMES: [&str; 11] = ["id", "title", "text", "kind", "stream", "tags", "meta", "created", "loc", "supersedes", "situations"];
        let found: Vec<Field> = NAMES.iter().map(|name| s.get_field(name)).collect::<Result<_, _>>()?;
        let [id, title, text, kind, stream, tags, meta, created, loc, supersedes, situations]: [Field; 11] =
            found.try_into().map_err(|_| anyhow::anyhow!("index schema is missing fields"))?;
        Ok(Self { id, title, text, kind, stream, tags, meta, created, loc, supersedes, situations })
    }
}

impl VaultIndex {
    /// Does no traversal or indexing. An existing schema mismatch is an error.
    /// The index directory must be outside the source vault, including through symlinks.
    pub fn open(root: &Path, kind: VaultType, index_dir: &Path) -> Result<Self> {
        let root = root.canonicalize()?;
        let index_dir = separate_index_dir(&root, index_dir)?;
        let schema = Fields::schema();
        let index = if index_dir.join("meta.json").exists() {
            Index::open_in_dir(&index_dir)?
        } else {
            Index::create_in_dir(&index_dir, schema.clone())?
        };
        if index.schema() != schema {
            bail!("index schema differs; rebuild into a new directory");
        }
        let fields = Fields::from_schema(&schema)?;
        let out = Self {
            index,
            root,
            kind,
            fields,
        };
        // Refuse an index accidentally routed to a different source vault.
        out.checkpoint()?;
        Ok(out)
    }

    fn document(&self, record: &Record, stream: &str, loc: &Location) -> Result<TantivyDocument> {
        let f = &self.fields;
        let mut d = TantivyDocument::default();
        d.add_text(f.id, &record.id);
        d.add_text(f.title, record.title.as_deref().unwrap_or(""));
        d.add_text(f.text, &record.text);
        d.add_text(f.kind, &record.kind);
        d.add_text(f.stream, stream);
        d.add_date(
            f.created,
            tantivy::DateTime::from_timestamp_secs(record.created.timestamp()),
        );
        for tag in &record.tags {
            d.add_text(f.tags, tag);
        }
        for old in &record.supersedes {
            d.add_text(f.supersedes, old);
        }
        for situation in cv_engine::situations::situations_of(Some(&serde_json::Value::Object(record.meta.clone()))) {
            d.add_text(f.situations, situation);
        }
        for (key, value) in &record.meta {
            if value.is_array() || value.is_object() {
                continue;
            }
            d.add_text(f.meta, meta_term(key, value)?);
        }
        d.add_text(f.loc, serde_json::to_string(loc)?);
        Ok(d)
    }

    pub fn count(&self) -> Result<u64> {
        Ok(self.index.reader()?.searcher().num_docs())
    }

    fn read_location(&self, loc: &Location) -> Result<Record> {
        match loc {
            Location::Markdown { path } => {
                if path.is_absolute()
                    || path
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    bail!("unsafe source location");
                }
                let path = self.root.join(path).canonicalize()?;
                if !path.starts_with(&self.root) {
                    bail!("source location leaves vault");
                }
                cv_core::markdown::read_record(&path)
            }
            Location::Segment { loc } => cv_core::read_at(&self.root, loc),
        }
    }
}

pub(crate) fn meta_term(key: &str, value: &serde_json::Value) -> Result<String> {
    Ok(serde_json::to_string(&(key, value))?)
}

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;

fn separate_index_dir(root: &Path, index_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(index_dir)?;
    let index_dir = index_dir.canonicalize()?;
    if index_dir.starts_with(root) || root.starts_with(&index_dir) {
        bail!("index and source directories must be separate");
    }
    Ok(index_dir)
}
