//! A v5 record as the v4 `Entry` the MCP tools and CLI already speak. Fields v5 does not
//! keep (hit counts, heat, summaries) are empty; frontmatter-only fields are read from the file.
use cv_core::Record;
use cv_engine::Entry;
use cv_vault::{parse_frontmatter, VaultTaxonomy};
use serde_json::Value;
use std::path::Path;

pub(crate) fn to_entry(record: Record, file: Option<&Path>, taxonomy: &VaultTaxonomy) -> Entry {
    let front = file.and_then(|f| std::fs::read_to_string(f).ok()).map(|raw| parse_frontmatter(&raw).0);
    let text = |key: &str| front.as_ref()?.get(key)?.as_str().map(str::to_string);
    let list = |key: &str| -> Option<Vec<String>> {
        let items = front.as_ref()?.get(key)?.as_array()?;
        Some(items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
    };
    let created = record.created.to_rfc3339();
    Entry {
        category: taxonomy.category_for(&record.kind).to_string(),
        tier: text("tier").unwrap_or_else(|| taxonomy.tier_for(&record.kind).to_string()),
        updated_at: text("updated").unwrap_or_else(|| created.clone()),
        identity_key: text("identity_key"),
        expires_at: text("expires_at"),
        related_to: list("related_to"),
        supersedes: (!record.supersedes.is_empty()).then(|| record.supersedes.clone()),
        meta: (!record.meta.is_empty()).then(|| Value::Object(record.meta.clone())),
        file_path: file.map(|f| f.to_string_lossy().into_owned()),
        id: record.id,
        kind: record.kind,
        title: record.title,
        body: record.text,
        tags: record.tags,
        source: record.source,
        indexed: true,
        created_at: created,
        superseded_by: text("superseded_by"),
        hit_count: 0,
        recall_count: 0,
        recall_sessions: 0,
        last_recalled_at: None,
        heat_tier: None,
        summary_condensed: None,
        summary_keypoint: None,
        last_accessed_at: None,
        source_files: None,
    }
}
