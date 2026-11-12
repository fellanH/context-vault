//! BM25 search over the vaults searched by default, merged by score.
use super::{Vault, V5Backend};
use crate::engine_entry::to_entry;
use crate::SearchOptions as IndexSearch;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use cv_core::VaultType;
use cv_engine::{Entry, ListOptions, ScoredEntry, SearchOptions};
use std::path::PathBuf;

pub(super) fn run(engine: &V5Backend, query: &str, opts: &SearchOptions) -> Result<Vec<ScoredEntry>> {
    let wanted = if opts.limit == 0 { 10 } else { opts.limit };
    let filter = Filter::new(opts)?;
    let mut hits = Vec::new();
    for vault in engine.vaults.iter().filter(|v| v.entry.search_by_default) {
        engine.catch_up(vault)?;
        hits.extend(vault_hits(engine, vault, query, opts, &filter, wanted + opts.offset)?);
    }
    hits.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(hits.into_iter().skip(opts.offset).take(wanted).collect())
}

/// The index can't apply every filter, so fetch more until the page is full or the matches end.
const MAX_FETCH: usize = 1000;

fn vault_hits(engine: &V5Backend, vault: &Vault, query: &str, opts: &SearchOptions, filter: &Filter, need: usize) -> Result<Vec<ScoredEntry>> {
    let mut fetch = need.max(1);
    loop {
        let limit = fetch.min(MAX_FETCH);
        let options = IndexSearch { kind: opts.kind.clone(), tag: filter.tags.first().cloned(), since: filter.since, limit, ..Default::default() };
        let raw = vault.index.search(query, &options)?;
        let exhausted = raw.len() < limit || limit == MAX_FETCH;
        let kept = keep_live(engine, vault, raw, opts, filter)?;
        if kept.len() >= need || exhausted {
            return Ok(kept);
        }
        fetch *= 4;
    }
}

fn keep_live(engine: &V5Backend, vault: &Vault, raw: Vec<crate::Hit>, opts: &SearchOptions, filter: &Filter) -> Result<Vec<ScoredEntry>> {
    let mut out = Vec::new();
    for hit in raw.into_iter().filter(|hit| filter.keeps(vault, &hit.record)) {
        let file = hit_file(vault, &hit.record);
        let mut entry = to_entry(hit.record, file.as_deref(), &vault.taxonomy);
        if entry.superseded_by.is_none() {
            entry.superseded_by = engine.superseder(&entry.id)?;
        }
        if opts.include_superseded || filter.is_live(&entry) {
            out.push(ScoredEntry { entry, score: hit.score, fts_rank: Some(hit.score), vec_rank: None });
        }
    }
    Ok(out)
}

/// Newest first in the default vault; category and tier are filtered before the page is cut.
pub(super) fn list(engine: &V5Backend, opts: &ListOptions) -> Result<Vec<Entry>> {
    let vault = &engine.vaults[engine.default];
    engine.catch_up(vault)?;
    let wanted = if opts.limit == 0 { 20 } else { opts.limit };
    let need = wanted + opts.offset;
    let mut fetch = need;
    loop {
        let records = vault.index.recent(opts.kind.as_deref(), fetch, 0)?;
        let exhausted = records.len() < fetch;
        let kept: Vec<Entry> = records
            .into_iter()
            .map(|(record, location)| to_entry(record, vault.index.source_path(&location).as_deref(), &vault.taxonomy))
            .filter(|e| opts.category.as_deref().is_none_or(|c| c == e.category) && opts.tier.as_deref().is_none_or(|t| t == e.tier))
            .collect();
        if kept.len() >= need || exhausted {
            return Ok(kept.into_iter().skip(opts.offset).take(wanted).collect());
        }
        fetch *= 4;
    }
}

/// The v4 options the index can't apply itself: category, every tag, `until`.
struct Filter {
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
    tags: Vec<String>,
    category: Option<String>,
    now: DateTime<Utc>,
}

impl Filter {
    fn new(opts: &SearchOptions) -> Result<Self> {
        Ok(Self {
            since: opts.since.as_deref().map(parse_time).transpose()?,
            until: opts.until.as_deref().map(parse_time).transpose()?,
            tags: opts.tags.clone().unwrap_or_default(),
            category: opts.category.clone(),
            now: Utc::now(),
        })
    }

    fn keeps(&self, vault: &Vault, record: &cv_core::Record) -> bool {
        let category = vault.taxonomy.category_for(&record.kind);
        self.category.as_deref().is_none_or(|c| c == category)
            && self.tags.iter().all(|t| record.tags.contains(t))
            && self.until.is_none_or(|u| record.created <= u)
    }

    /// Card 19's rule, as in v4: superseded or expired records leave default results.
    fn is_live(&self, entry: &cv_engine::Entry) -> bool {
        let expired = entry.expires_at.as_deref().and_then(|t| parse_time(t).ok()).is_some_and(|t| t <= self.now);
        entry.superseded_by.is_none() && !expired
    }
}

/// Search hits carry the record, not its location; a knowledge record's file is found by id.
fn hit_file(vault: &Vault, record: &cv_core::Record) -> Option<PathBuf> {
    if vault.entry.kind != VaultType::Knowledge {
        return None;
    }
    let (_, location) = vault.index.get(&record.id).ok()??;
    vault.index.source_path(&location)
}

fn parse_time(raw: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    if let Ok(t) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Ok(t.with_timezone(&chrono::Utc));
    }
    let day = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| anyhow!("not a date: {raw}"))?;
    Ok(day.and_hms_opt(0, 0, 0).expect("midnight").and_utc())
}

