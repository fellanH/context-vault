//! WHERE-clause fragments built from `SearchOptions` filters, shared by the FTS
//! and vector search queries in `queries.rs`.

use cv_engine::SearchOptions;

/// Extra `AND v.<col> = ...` predicates built from the SearchOptions filters, to be
/// appended to a search WHERE clause. Values are single-quote-escaped inline (same
/// pattern as `sqlite.rs::build_list_query`); the SELECT column list is untouched so
/// `COL_SCORE` stays valid. `tags` matches against the JSON-array text column.
pub(crate) fn filter_clause(opts: &SearchOptions) -> String {
    let esc = |s: &str| s.replace('\'', "''");
    let mut c = String::new();
    if let Some(kind) = &opts.kind {
        c.push_str(&format!(" AND v.kind = '{}'", esc(kind)));
    }
    if let Some(category) = &opts.category {
        c.push_str(&format!(" AND v.category = '{}'", esc(category)));
    }
    if let Some(tier) = &opts.tier {
        c.push_str(&format!(" AND v.tier = '{}'", esc(tier)));
    }
    if let Some(since) = &opts.since {
        c.push_str(&format!(" AND v.created_at >= '{}'", esc(since)));
    }
    if let Some(until) = &opts.until {
        c.push_str(&format!(" AND v.created_at <= '{}'", esc(until)));
    }
    if let Some(tags) = &opts.tags {
        c.push_str(&tag_filters(tags, &esc));
    }
    if let Some(project) = &opts.scope_project {
        c.push_str(&scope_project_filter(project, &esc));
    }
    if !opts.include_superseded {
        c.push_str(LIVE_ONLY);
    }
    c
}

/// Drops superseded and expired rows. `expires_at` is stored as ISO-8601 with a `T`,
/// so `now` is formatted the same way; `datetime('now')` uses a space and would keep
/// a row alive until the end of the day it expired.
const LIVE_ONLY: &str = " AND v.superseded_by IS NULL \
    AND (v.expires_at IS NULL OR v.expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))";

/// Project-scope predicate: an entry passes when it carries NO project-identity
/// tags (`bucket:*` / `seat:*`) — including the ~half of real vaults where `tags`
/// is NULL — or when one of them names this project. Keeps cross-project entries
/// out of scoped searches while global (untagged) knowledge stays reachable.
fn scope_project_filter(project: &str, esc: &impl Fn(&str) -> String) -> String {
    let p = esc(project);
    format!(
        " AND (v.tags IS NULL \
         OR (v.tags NOT LIKE '%\"bucket:%' AND v.tags NOT LIKE '%\"seat:%') \
         OR v.tags LIKE '%\"bucket:{p}\"%' OR v.tags LIKE '%\"seat:{p}\"%')"
    )
}

/// One `AND v.tags LIKE '%"tag"%'` per required tag (tags stored as a JSON array text).
fn tag_filters(tags: &[String], esc: &impl Fn(&str) -> String) -> String {
    tags.iter()
        .map(|t| format!(" AND v.tags LIKE '%\"{}\"%'", esc(t)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_project_includes_null_untagged_and_own_project_tags() {
        let opts = SearchOptions { scope_project: Some("glance".to_string()), ..Default::default() };
        let clause = filter_clause(&opts);
        // NULL tags (about half of real vaults) and untagged entries must pass;
        // only the scoped project's bucket:/seat: tags match.
        assert!(clause.contains("v.tags IS NULL"));
        assert!(clause.contains("NOT LIKE '%\"bucket:%'"));
        assert!(clause.contains("'%\"bucket:glance\"%'"));
        assert!(clause.contains("'%\"seat:glance\"%'"));
    }
}
