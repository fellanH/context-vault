//! `context-vault stale`: entries older than their kind's `fresh_days` window (card 28).
//! Age counts from `verified_at` (set by `verify`), else `updated`, else `created`.

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};

use crate::export::{collect, ExportRow};
use crate::VaultTaxonomy;

pub struct StaleEntry {
    pub kind: String,
    pub id: Option<String>,
    pub title: Option<String>,
    pub last_checked: String,
    pub days_old: i64,
    pub fresh_days: u32,
}

/// Kinds to check: the one asked for, else every kind whose rule sets `fresh_days`.
pub fn stale(vault_dir: &str, taxonomy: &VaultTaxonomy, kind: Option<&str>, now: DateTime<Utc>) -> Result<Vec<StaleEntry>> {
    let kinds: Vec<String> = match kind {
        Some(k) => vec![k.to_string()],
        None => taxonomy.kinds().iter().filter(|(_, r)| r.fresh_days.is_some()).map(|(k, _)| k.clone()).collect(),
    };
    let mut out = vec![];
    for k in kinds {
        let Some(window) = taxonomy.rule_for(&k).fresh_days else { continue };
        let (_, rows) = collect(vault_dir, taxonomy, &k)?;
        out.extend(rows.into_iter().filter_map(|r| stale_entry(&k, r, window, now)));
    }
    out.sort_by_key(|entry| std::cmp::Reverse(entry.days_old));
    Ok(out)
}

fn stale_entry(kind: &str, row: ExportRow, window: u32, now: DateTime<Utc>) -> Option<StaleEntry> {
    let verified = row.meta.get("verified_at").and_then(|v| v.as_str()).map(|s| s.to_string());
    let last = verified.or(row.updated).or(row.created)?;
    let days_old = (now - parse_time(&last)?).num_days();
    (days_old > i64::from(window)).then(|| StaleEntry {
        kind: kind.to_string(),
        id: row.id,
        title: row.title,
        last_checked: last,
        days_old,
        fresh_days: window,
    })
}

/// RFC 3339 timestamps, or a bare `YYYY-MM-DD` date (midnight UTC).
fn parse_time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .ok()
        .or_else(|| NaiveDate::parse_from_str(s.get(..10)?, "%Y-%m-%d").ok()?.and_hms_opt(0, 0, 0).map(|t| t.and_utc()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_entries_past_their_window_counting_from_verified_at() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join("vault.toml"), "schema_version = 1\n[default]\ncategory = \"k\"\ntier = \"working\"\nidentity = false\n[kinds.price]\ncategory = \"data\"\ntier = \"durable\"\nfresh_days = 30\n[kinds.note]\ncategory = \"k\"\ntier = \"working\"\n").unwrap();
        std::fs::create_dir_all(dir.join("data/price")).unwrap();
        std::fs::create_dir_all(dir.join("k/note")).unwrap();
        let w = |name: &str, fm: &str| std::fs::write(dir.join("data/price").join(name), format!("---\n{fm}\n---\nx\n")).unwrap();
        w("old.md", "id: OLD\ntitle: Old price\nupdated: 2026-01-01T00:00:00Z");
        w("checked.md", "id: CHK\ntitle: Re-checked\nupdated: 2026-01-01T00:00:00Z\nverified_at: 2026-10-10T00:00:00Z");
        w("fresh.md", "id: NEW\ntitle: New price\ncreated: 2026-10-01");
        std::fs::write(dir.join("k/note/n.md"), "---\nid: N\nupdated: 2020-01-01T00:00:00Z\n---\nx\n").unwrap();
        let tax = VaultTaxonomy::load_strict(dir.to_str().unwrap()).unwrap();
        let now = DateTime::parse_from_rfc3339("2026-10-19T00:00:00Z").unwrap().with_timezone(&Utc);

        let found = stale(dir.to_str().unwrap(), &tax, None, now).unwrap();
        let ids: Vec<_> = found.iter().map(|e| e.id.clone().unwrap()).collect();
        assert_eq!(ids, vec!["OLD"], "only the unverified old price; notes have no window");
        assert_eq!(found[0].fresh_days, 30);
        assert!(stale(dir.to_str().unwrap(), &tax, Some("note"), now).unwrap().is_empty(), "no window, nothing stale");
    }
}
