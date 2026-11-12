//! Query assembly keeps user text separate from exact structured filters.
use crate::{meta_term, Hit, SearchOptions, VaultIndex};
use anyhow::{anyhow, bail, Result};
use tantivy::{
    collector::TopDocs,
    query::{AllQuery, BooleanQuery, Occur, Query, QueryParser, RangeQuery, TermQuery},
    schema::{IndexRecordOption, Value},
    TantivyDocument, Term,
};
type Clause = (Occur, Box<dyn Query>);
impl VaultIndex {
    pub fn search(&self, text: &str, options: &SearchOptions) -> Result<Vec<Hit>> {
        let limit = if options.limit == 0 {
            10
        } else {
            options.limit
        };
        if limit > 1000 {
            bail!("limit must be at most 1000");
        }
        let query = build_query(self, text, options)?;
        let searcher = self.index.reader()?.searcher();
        let results = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score())?;
        results
            .into_iter()
            .map(|(score, address)| {
                let document: TantivyDocument = searcher.doc(address)?;
                read_hit(self, &document, score)
            })
            .collect()
    }
}
fn build_query(index: &VaultIndex, text: &str, options: &SearchOptions) -> Result<BooleanQuery> {
    let parser = QueryParser::for_index(&index.index, vec![index.fields.title, index.fields.text]);
    let base: Box<dyn Query> = if text.trim().is_empty() {
        Box::new(AllQuery)
    } else {
        // Agents type plain text ("card 46: ids (v5)"): syntax errors degrade to terms, never fail.
        parser.parse_query_lenient(text).0
    };
    let mut clauses = vec![(Occur::Must, base)];
    clauses.extend(scalar_filters(index, options));
    clauses.extend(meta_filters(index, options)?);
    if let Some(query) = since_filter(index, options) {
        clauses.push((Occur::Must, query));
    }
    Ok(BooleanQuery::new(clauses))
}
fn term_filter(field: tantivy::schema::Field, value: &str) -> Clause {
    (
        Occur::Must,
        Box::new(TermQuery::new(
            Term::from_field_text(field, value),
            IndexRecordOption::Basic,
        )),
    )
}
fn scalar_filters(index: &VaultIndex, options: &SearchOptions) -> Vec<Clause> {
    [
        (index.fields.kind, &options.kind),
        (index.fields.stream, &options.stream),
        (index.fields.tags, &options.tag),
    ]
    .into_iter()
    .filter_map(|(field, value)| value.as_deref().map(|value| term_filter(field, value)))
    .collect()
}
fn meta_filters(index: &VaultIndex, options: &SearchOptions) -> Result<Vec<Clause>> {
    options
        .meta
        .iter()
        .map(|(key, value)| {
            if value.is_array() || value.is_object() {
                bail!("meta filters must be scalar");
            }
            Ok(term_filter(index.fields.meta, &meta_term(key, value)?))
        })
        .collect()
}
fn since_filter(index: &VaultIndex, options: &SearchOptions) -> Option<Box<dyn Query>> {
    use std::ops::Bound;
    options.since.map(|since| {
        let term = Term::from_field_date(
            index.fields.created,
            tantivy::DateTime::from_timestamp_secs(since.timestamp()),
        );
        Box::new(RangeQuery::new(Bound::Included(term), Bound::Unbounded)) as Box<dyn Query>
    })
}
fn read_hit(index: &VaultIndex, document: &TantivyDocument, score: f32) -> Result<Hit> {
    let id = document
        .get_first(index.fields.id)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("index record has no id"))?;
    let raw = document
        .get_first(index.fields.loc)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("index record has no location"))?;
    let record = index.read_location(&serde_json::from_str(raw)?)?;
    if record.id != id {
        bail!("source changed after indexing; run catch-up");
    }
    Ok(Hit { score, record })
}
