//! Reads that are not ranked searches: one record by id, and the newest records of a kind.
use crate::{Location, VaultIndex};
use anyhow::{anyhow, Result};
use tantivy::{
    collector::TopDocs,
    query::{AllQuery, BooleanQuery, Occur, Query, TermQuery},
    schema::{IndexRecordOption, Value},
    Order, TantivyDocument, Term,
};

impl VaultIndex {
    /// The record with this id and where its source lives, read from the source (not the index).
    pub fn get(&self, id: &str) -> Result<Option<(cv_core::Record, Location)>> {
        let query = TermQuery::new(Term::from_field_text(self.fields.id, id), IndexRecordOption::Basic);
        let searcher = self.index.reader()?.searcher();
        let Some((_, address)) = searcher.search(&query, &TopDocs::with_limit(1).order_by_score())?.into_iter().next() else {
            return Ok(None);
        };
        let location = self.location_of(&searcher.doc(address)?)?;
        let record = self.read_location(&location)?;
        Ok((record.id == id).then_some((record, location)))
    }

    /// Records newest first (by `created`), optionally of one kind.
    pub fn recent(&self, kind: Option<&str>, limit: usize, offset: usize) -> Result<Vec<(cv_core::Record, Location)>> {
        let query: Box<dyn Query> = match kind {
            Some(kind) => Box::new(BooleanQuery::new(vec![(
                Occur::Must,
                Box::new(TermQuery::new(Term::from_field_text(self.fields.kind, kind), IndexRecordOption::Basic)) as Box<dyn Query>,
            )])),
            None => Box::new(AllQuery),
        };
        let collector = TopDocs::with_limit(limit.max(1)).and_offset(offset).order_by_fast_field::<tantivy::DateTime>("created", Order::Desc);
        let searcher = self.index.reader()?.searcher();
        let mut out = Vec::new();
        for (_, address) in searcher.search(&query, &collector)? {
            let location = self.location_of(&searcher.doc(address)?)?;
            // A source edited since the last catch-up is skipped, not shown stale.
            if let Ok(record) = self.read_location(&location) {
                out.push((record, location));
            }
        }
        Ok(out)
    }

    /// Live records per kind (a count per kind term, so deleted documents are not counted).
    pub fn kind_counts(&self) -> Result<std::collections::BTreeMap<String, u64>> {
        let searcher = self.index.reader()?.searcher();
        let mut kinds = std::collections::BTreeSet::new();
        for segment in searcher.segment_readers() {
            let inverted = segment.inverted_index(self.fields.kind)?;
            let mut terms = inverted.terms().stream()?;
            while terms.advance() {
                kinds.insert(String::from_utf8_lossy(terms.key()).into_owned());
            }
        }
        let mut counts = std::collections::BTreeMap::new();
        for kind in kinds {
            let query = TermQuery::new(Term::from_field_text(self.fields.kind, &kind), IndexRecordOption::Basic);
            let n = searcher.search(&query, &tantivy::collector::Count)? as u64;
            if n > 0 {
                counts.insert(kind, n);
            }
        }
        Ok(counts)
    }

    /// The id of another record that lists `id` in its `supersedes`, if any (never itself).
    pub fn superseded_by(&self, id: &str) -> Result<Option<String>> {
        let query = TermQuery::new(Term::from_field_text(self.fields.supersedes, id), IndexRecordOption::Basic);
        let searcher = self.index.reader()?.searcher();
        for (_, address) in searcher.search(&query, &TopDocs::with_limit(2).order_by_score())? {
            let document: TantivyDocument = searcher.doc(address)?;
            let newer = document.get_first(self.fields.id).and_then(|v| v.as_str());
            if let Some(newer) = newer.filter(|newer| *newer != id) {
                return Ok(Some(newer.to_string()));
            }
        }
        Ok(None)
    }

    pub(crate) fn location_of(&self, document: &TantivyDocument) -> Result<Location> {
        let raw = document.get_first(self.fields.loc).and_then(|v| v.as_str()).ok_or_else(|| anyhow!("index record has no location"))?;
        Ok(serde_json::from_str(raw)?)
    }

    /// Absolute path of a markdown source.
    pub fn source_path(&self, location: &Location) -> Option<std::path::PathBuf> {
        match location {
            Location::Markdown { path } => Some(self.root.join(path)),
            Location::Segment { .. } => None,
        }
    }

    /// Bytes the index's live segments take (tantivy's own accounting).
    pub fn size_bytes(&self) -> Result<u64> {
        Ok(self.index.reader()?.searcher().space_usage()?.total().get_bytes())
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}
