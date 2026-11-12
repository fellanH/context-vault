use std::fs;
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::{Map, Value};
use ulid::Ulid;

use cv_engine::SaveInput;
use crate::{
    fm_parse::parse_frontmatter,
    entry_file::{write_entry_file, write_entry_file_at},
    schema_check::check_save,
    taxonomy::VaultTaxonomy,
};

#[derive(Debug, Clone)]
pub struct WriteResult {
    pub id: String,
    pub file_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub category: String,
    pub tier: String,
}

#[derive(Clone, Debug)]
pub struct WriteEntryParams {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub meta: Option<Option<Map<String, Value>>>,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub folder: Option<String>,
    pub category: String,
    pub identity_key: Option<String>,
    pub expires_at: Option<String>,
    pub supersedes: Option<Vec<String>>,
    pub related_to: Option<Vec<String>>,
    pub tier: Option<String>,
}

fn resolve_entity_identity(
    vault_dir: &str,
    taxonomy: &VaultTaxonomy,
    kind: &str,
    identity_key: &Option<String>,
) -> Result<(String, String, String)> {
    use crate::files::slugify;

    let identity_key = match identity_key {
        Some(k) if !k.is_empty() => k,
        _ => {
            let id = Ulid::new().to_string();
            let now = Utc::now().to_rfc3339();
            return Ok((id.clone(), now.clone(), now));
        }
    };

    let identity_slug = slugify(identity_key, 60);
    let vault_path = PathBuf::from(vault_dir);
    let dir = vault_path.join(taxonomy.folder_for(kind));
    let existing_path = dir.join(format!("{}.md", identity_slug));

    if existing_path.exists() {
        let file_content = fs::read_to_string(&existing_path)?;
        let (fm_meta, _) = parse_frontmatter(&file_content);
        let id = fm_meta
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Ulid::new().to_string());
        let created = fm_meta
            .get("created")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        let updated = Utc::now().to_rfc3339();
        Ok((id, created, updated))
    } else {
        let id = Ulid::new().to_string();
        let now = Utc::now().to_rfc3339();
        Ok((id, now.clone(), now))
    }
}

/// Resolved identity + timestamps + classification for a new/updated entry.
struct ResolvedWriteFields {
    id: String,
    created_at: String,
    updated_at: String,
    category: String,
    tier: String,
}

fn resolve_write_fields(
    vault_dir: &str,
    taxonomy: &VaultTaxonomy,
    input: &SaveInput,
) -> Result<ResolvedWriteFields> {
    let category = taxonomy.category_for(&input.kind).to_string();
    let tier = input
        .tier
        .clone()
        .unwrap_or_else(|| taxonomy.tier_for(&input.kind).to_string());

    let (id, created_at, updated_at) = if taxonomy.is_identity(&input.kind) {
        resolve_entity_identity(vault_dir, taxonomy, &input.kind, &input.identity_key)?
    } else {
        let id = Ulid::new().to_string();
        let now = Utc::now().to_rfc3339();
        (id, now.clone(), now)
    };

    Ok(ResolvedWriteFields { id, created_at, updated_at, category, tier })
}

fn check_input(input: &SaveInput) -> Result<()> {
    if input.kind.is_empty() {
        return Err(anyhow!("writeEntry: kind is required (non-empty string)"));
    }
    if input.body.trim().is_empty() {
        return Err(anyhow!("writeEntry: body is required (non-empty string)"));
    }
    Ok(())
}

fn write_params(input: &SaveInput, f: &ResolvedWriteFields) -> WriteEntryParams {
    WriteEntryParams {
        id: f.id.clone(),
        title: input.title.clone(),
        body: input.body.clone(),
        meta: Some(input.meta.as_ref().and_then(|v| v.as_object().cloned())),
        tags: input.tags.clone(),
        source: input.source.clone(),
        created_at: f.created_at.clone(),
        updated_at: f.updated_at.clone(),
        folder: None,
        category: f.category.clone(),
        identity_key: input.identity_key.clone(),
        expires_at: input.expires_at.clone(),
        supersedes: input.supersedes.clone(),
        related_to: input.related_to.clone(),
        tier: Some(f.tier.clone()),
    }
}

/// Rewrites an existing entry in place: same id, created time and file path.
pub fn rewrite_entry(vault_dir: &str, input: &SaveInput, id: &str, created_at: &str, file_path: &str) -> Result<WriteResult> {
    check_input(input)?;
    let taxonomy = VaultTaxonomy::load_strict(vault_dir)?;
    check_save(&taxonomy, input)?;
    let fields = ResolvedWriteFields {
        id: id.to_string(),
        created_at: created_at.to_string(),
        updated_at: Utc::now().to_rfc3339(),
        category: taxonomy.category_for(&input.kind).to_string(),
        tier: input.tier.clone().unwrap_or_else(|| taxonomy.tier_for(&input.kind).to_string()),
    };
    let file_path = write_entry_file_at(file_path, &input.kind, write_params(input, &fields))?;
    let ResolvedWriteFields { id, created_at, updated_at, category, tier } = fields;
    Ok(WriteResult { id, file_path, created_at, updated_at, category, tier })
}

pub fn write_entry(vault_dir: &str, input: &SaveInput) -> Result<WriteResult> {
    check_input(input)?;

    let taxonomy = VaultTaxonomy::load_strict(vault_dir)?;
    check_save(&taxonomy, input)?;
    let fields = resolve_write_fields(vault_dir, &taxonomy, input)?;
    let file_path = write_entry_file(vault_dir, &input.kind, &taxonomy, write_params(input, &fields))?;
    let ResolvedWriteFields { id, created_at, updated_at, category, tier } = fields;

    Ok(WriteResult {
        id,
        file_path,
        created_at,
        updated_at,
        category,
        tier,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_entry_filename_entity() {
        let temp_dir = TempDir::new().unwrap();
        let vault_dir = temp_dir.path().to_str().unwrap();

        // identity-based upsert is opt-in via vault.toml, not assumed for any kind name.
        std::fs::write(
            temp_dir.path().join("vault.toml"),
            r#"
schema_version = 1
[default]
category = ""
tier = "working"
[kinds.contact]
category = "entities"
identity = true
"#,
        )
        .unwrap();

        let input = SaveInput {
            id: Some("test-id-123".to_string()),
            kind: "contact".to_string(),
            title: Some("John Doe".to_string()),
            body: "Contact info".to_string(),
            tags: Some(vec!["person".to_string()]),
            meta: None,
            source: None,
            identity_key: Some("john-doe".to_string()),
            tier: None,
            expires_at: None,
            supersedes: None,
            related_to: None,
        };

        let result = write_entry(vault_dir, &input).unwrap();
        assert!(!result.file_path.is_empty());
        assert!(result.file_path.contains("john-doe.md"));
        assert!(result.file_path.contains("entities"));
    }

    #[test]
    fn test_write_entry_filename_no_config_is_flat() {
        let temp_dir = TempDir::new().unwrap();
        let vault_dir = temp_dir.path().to_str().unwrap();

        let input = SaveInput {
            id: Some("short-id-12345678".to_string()),
            kind: "insight".to_string(),
            title: Some("Some Insight Title".to_string()),
            body: "Body text".to_string(),
            tags: None,
            meta: None,
            source: None,
            identity_key: None,
            tier: None,
            expires_at: None,
            supersedes: None,
            related_to: None,
        };

        let result = write_entry(vault_dir, &input).unwrap();
        assert!(!result.file_path.is_empty());
        // no vault.toml -> zero-config default: flat folder named after the kind, not
        // any implicit "knowledge" grouping.
        assert!(result.file_path.contains("/insight/"));
        assert!(!result.file_path.contains("knowledge"));
        assert!(result.file_path.contains(".md"));
    }
}
