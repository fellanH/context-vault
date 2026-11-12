use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use serde_json::{Map, Value};

use crate::{
    files::slugify,
    formatters::{format_body, FormatInput},
    frontmatter::format_frontmatter,
    taxonomy::VaultTaxonomy,
};
use crate::capture::WriteEntryParams;

/// Last 8 chars of the id, lowercased — matches TS `id.slice(-8).toLowerCase()`.
fn short_id_lower(id: &str) -> String {
    let start = id.len().saturating_sub(8);
    id[start..].to_lowercase()
}

fn resolve_folder(folder: &Option<String>, meta: &Option<Option<Map<String, Value>>>) -> String {
    if let Some(f) = folder.as_ref().filter(|s| !s.is_empty()) {
        return f.clone();
    }
    folder_from_meta(meta).unwrap_or_default()
}

/// Pull a non-empty `folder` string out of the optional custom-meta map. Uses `?`
/// chaining (no branch penalty) to stay flat under the complexity ceiling.
fn folder_from_meta(meta: &Option<Option<Map<String, Value>>>) -> Option<String> {
    let map = meta.as_ref()?.as_ref()?;
    let f = map.get("folder")?.as_str()?;
    if f.is_empty() {
        None
    } else {
        Some(f.to_string())
    }
}

fn compute_filename(
    is_identity: bool,
    identity_key: &Option<String>,
    id: &str,
    title: &Option<String>,
    body: &str,
) -> String {
    if is_identity && identity_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false) {
        let identity_key = identity_key.as_ref().unwrap();
        let identity_slug = slugify(identity_key, 60);
        if !identity_slug.is_empty() {
            return format!("{}.md", identity_slug);
        }
        return format!("{}.md", short_id_lower(id));
    }

    let text_for_slug = if let Some(title) = title {
        title
    } else {
        body
    };
    let slug = slugify(&text_for_slug.chars().take(40).collect::<String>(), 60);
    let short_id = short_id_lower(id);

    if !slug.is_empty() {
        format!("{}-{}.md", slug, short_id)
    } else {
        format!("{}.md", short_id)
    }
}

fn build_fm_fields(
    kind: &str,
    _category: &str,
    params: &WriteEntryParams,
) -> Vec<(String, Value)> {
    let mut fields: Vec<(String, Value)> = Vec::new();

    fields.push(("id".to_string(), Value::String(params.id.clone())));
    if let Some(title) = &params.title {
        if !title.is_empty() {
            fields.push(("title".to_string(), Value::String(title.clone())));
        }
    }
    fields.push(("kind".to_string(), Value::String(kind.to_string())));
    if let Some(tier) = &params.tier {
        if !tier.is_empty() {
            fields.push(("tier".to_string(), Value::String(tier.clone())));
        }
    }

    if let Some(Some(meta)) = &params.meta {
        for (k, v) in meta {
            if k != "folder" && !v.is_null() {
                fields.push((k.clone(), v.clone()));
            }
        }
    }

    if let Some(identity_key) = &params.identity_key {
        if !identity_key.is_empty() {
            fields.push(("identity_key".to_string(), Value::String(identity_key.clone())));
        }
    }
    if let Some(expires_at) = &params.expires_at {
        if !expires_at.is_empty() {
            fields.push(("expires_at".to_string(), Value::String(expires_at.clone())));
        }
    }
    if let Some(supersedes) = &params.supersedes {
        if !supersedes.is_empty() {
            fields.push(("supersedes".to_string(), Value::Array(
                supersedes.iter().map(|s| Value::String(s.clone())).collect()
            )));
        }
    }
    if let Some(related_to) = &params.related_to {
        if !related_to.is_empty() {
            fields.push(("related_to".to_string(), Value::Array(
                related_to.iter().map(|s| Value::String(s.clone())).collect()
            )));
        }
    }

    let tags = params.tags.as_ref().cloned().unwrap_or_default();
    fields.push(("tags".to_string(), Value::Array(
        tags.iter().map(|t| Value::String(t.clone())).collect()
    )));

    let source = params.source.as_ref().cloned().unwrap_or_else(|| "claude-code".to_string());
    fields.push(("source".to_string(), Value::String(source)));

    fields.push(("created".to_string(), Value::String(params.created_at.clone())));
    if !params.updated_at.is_empty() && params.updated_at != params.created_at {
        fields.push(("updated".to_string(), Value::String(params.updated_at.clone())));
    }

    fields
}

pub fn write_entry_file(
    vault_dir: &str,
    kind: &str,
    taxonomy: &VaultTaxonomy,
    params: WriteEntryParams,
) -> Result<String> {
    let resolved_folder = resolve_folder(&params.folder, &params.meta);
    let base_dir = PathBuf::from(vault_dir).join(taxonomy.folder_for(kind));
    let dir = if resolved_folder.is_empty() {
        base_dir
    } else {
        base_dir.join(&resolved_folder)
    };

    fs::create_dir_all(&dir)?;

    let filename = compute_filename(
        taxonomy.is_identity(kind),
        &params.identity_key,
        &params.id,
        &params.title,
        &params.body,
    );

    let file_path = dir.join(&filename);
    fs::write(&file_path, render_entry_file(kind, &params))?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Rewrites an existing entry's file at its current path (update in place).
pub fn write_entry_file_at(file_path: &str, kind: &str, params: WriteEntryParams) -> Result<String> {
    if let Some(dir) = PathBuf::from(file_path).parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(file_path, render_entry_file(kind, &params))?;
    Ok(file_path.to_string())
}

fn render_entry_file(kind: &str, params: &WriteEntryParams) -> String {
    let fm_fields = build_fm_fields(kind, &params.category, params);
    let md_body = format_body(kind, FormatInput {
        title: params.title.clone(),
        body: params.body.clone(),
        meta: params.meta.clone().flatten(),
    });
    format_frontmatter(&fm_fields) + &md_body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_id_lower() {
        assert_eq!(short_id_lower("ABCDEFGH12345678"), "12345678");
        assert_eq!(short_id_lower("short"), "short");
    }

    #[test]
    fn test_resolve_folder() {
        let folder = Some("my-folder".to_string());
        assert_eq!(resolve_folder(&folder, &None), "my-folder");

        let folder = Some("".to_string());
        assert_eq!(resolve_folder(&folder, &None), "");
    }
}
