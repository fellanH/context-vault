//! Situation reads: the names in use and the records that carry one.
use crate::{Location, VaultIndex};
use anyhow::Result;
use std::collections::BTreeSet;
use tantivy::{collector::TopDocs, query::TermQuery, schema::IndexRecordOption, TantivyDocument, Term};

/// Situations are meant for a handful of entries each; past this a page set is cut.
const MAX_MEMBERS: usize = 1000;

impl VaultIndex {
    /// Every situation name some indexed record carries.
    pub fn situation_names(&self) -> Result<BTreeSet<String>> {
        let searcher = self.index.reader()?.searcher();
        let mut names = BTreeSet::new();
        for segment in searcher.segment_readers() {
            let inverted = segment.inverted_index(self.fields.situations)?;
            let mut terms = inverted.terms().stream()?;
            while terms.advance() {
                names.insert(String::from_utf8_lossy(terms.key()).into_owned());
            }
        }
        Ok(names)
    }

    /// The records carrying `situation`, read from their sources (unordered).
    pub fn situation_members(&self, situation: &str) -> Result<Vec<(cv_core::Record, Location)>> {
        let query = TermQuery::new(Term::from_field_text(self.fields.situations, situation), IndexRecordOption::Basic);
        let searcher = self.index.reader()?.searcher();
        let mut out = Vec::new();
        for (_, address) in searcher.search(&query, &TopDocs::with_limit(MAX_MEMBERS).order_by_score())? {
            let document: TantivyDocument = searcher.doc(address)?;
            let location = self.location_of(&document)?;
            if let Ok(record) = self.read_location(&location) {
                out.push((record, location));
            }
        }
        Ok(out)
    }
}
