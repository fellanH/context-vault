//! Reconciles existing markdown files with the current taxonomy after `vault.toml`
//! changes. Editing the config alone doesn't move anything — this walks the vault and
//! moves any file whose current directory no longer matches its kind's declared
//! folder. Safe by default: a move is skipped (not overwritten) if the destination
//! already exists, since that usually means a duplicate was already created by a save
//! that ran between the config edit and this apply (see specs/vault-taxonomy.md).

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Map, Value};

use crate::{
    files::{walk_dir, WalkResult},
    fm_parse::parse_frontmatter,
    taxonomy::VaultTaxonomy,
};

#[derive(Debug, Default)]
pub struct ReorgSummary {
    pub moved: Vec<(String, String)>,
    pub skipped: Vec<String>,
    pub unchanged: usize,
}

enum ReorgAction {
    Unchanged,
    Skip(String),
    Move(String, String),
}

fn kind_for_file(rel_dir: &str, fm: &Map<String, Value>) -> String {
    fm.get("kind")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            std::path::Path::new(rel_dir)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("note")
                .to_string()
        })
}

/// Decide what (if anything) needs to happen to bring one file in line with the
/// current taxonomy. Returns `None` for files that can't be read (skipped silently,
/// same as the old inline behavior).
fn plan_action(vault_dir: &str, taxonomy: &VaultTaxonomy, wr: &WalkResult) -> Option<ReorgAction> {
    let raw = fs::read_to_string(&wr.file_path).ok()?;
    let (fm, _body) = parse_frontmatter(&raw);
    let kind = kind_for_file(&wr.rel_dir, &fm);
    let expected_base = taxonomy.folder_for(&kind);

    // Already under the right base (possibly with an extra per-entry subfolder from
    // meta.folder) -- leave it alone.
    let already_placed =
        wr.rel_dir == expected_base || wr.rel_dir.starts_with(&format!("{}/", expected_base));
    if already_placed {
        return Some(ReorgAction::Unchanged);
    }

    let filename = PathBuf::from(&wr.file_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    let dest_path = PathBuf::from(vault_dir).join(&expected_base).join(&filename);

    if dest_path.exists() {
        return Some(ReorgAction::Skip(wr.file_path.clone()));
    }

    Some(ReorgAction::Move(
        wr.file_path.clone(),
        dest_path.to_string_lossy().to_string(),
    ))
}

fn execute_move(from: &str, to: &str) -> Result<()> {
    if let Some(parent) = PathBuf::from(to).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(from, to)?;
    Ok(())
}

/// `dry_run: true` computes the exact same plan (including the destination-conflict
/// check) but performs no filesystem writes -- safe to run against a live vault to
/// preview what `apply_reorg(vault_dir, false)` would do.
pub fn plan_reorg(vault_dir: &str, dry_run: bool) -> Result<ReorgSummary> {
    let taxonomy = VaultTaxonomy::load(vault_dir);
    let files = walk_dir(vault_dir)?;
    let mut summary = ReorgSummary::default();

    for wr in &files {
        match plan_action(vault_dir, &taxonomy, wr) {
            Some(ReorgAction::Unchanged) => summary.unchanged += 1,
            Some(ReorgAction::Skip(path)) => summary.skipped.push(path),
            Some(ReorgAction::Move(from, to)) => {
                if !dry_run {
                    execute_move(&from, &to)?;
                }
                summary.moved.push((from, to));
            }
            None => {}
        }
    }

    Ok(summary)
}

pub fn apply_reorg(vault_dir: &str) -> Result<ReorgSummary> {
    plan_reorg(vault_dir, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_entry(vault_dir: &std::path::Path, rel_dir: &str, filename: &str, kind: &str) {
        let dir = vault_dir.join(rel_dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(filename),
            format!("---\nid: test-id\nkind: {kind}\n---\nbody\n"),
        )
        .unwrap();
    }

    #[test]
    fn moves_file_to_match_updated_taxonomy() {
        let temp = TempDir::new().unwrap();
        let vault_dir = temp.path();
        write_entry(vault_dir, "entities/contact", "jane.md", "contact");

        fs::write(
            vault_dir.join("vault.toml"),
            r#"
schema_version = 1
[default]
category = "knowledge"
tier = "working"
[kinds.contact]
category = "people"
identity = true
"#,
        )
        .unwrap();

        let summary = apply_reorg(vault_dir.to_str().unwrap()).unwrap();
        assert_eq!(summary.moved.len(), 1);
        assert!(vault_dir.join("people/contact/jane.md").exists());
        assert!(!vault_dir.join("entities/contact/jane.md").exists());
    }

    #[test]
    fn skips_conflicting_destination_without_overwriting() {
        let temp = TempDir::new().unwrap();
        let vault_dir = temp.path();
        write_entry(vault_dir, "entities/contact", "jane.md", "contact");
        write_entry(vault_dir, "people/contact", "jane.md", "contact");

        fs::write(
            vault_dir.join("vault.toml"),
            r#"
schema_version = 1
[default]
category = "knowledge"
tier = "working"
[kinds.contact]
category = "people"
identity = true
"#,
        )
        .unwrap();

        let summary = apply_reorg(vault_dir.to_str().unwrap()).unwrap();
        assert_eq!(summary.moved.len(), 0);
        assert_eq!(summary.skipped.len(), 1);
        // both files still present -- no data loss.
        assert!(vault_dir.join("entities/contact/jane.md").exists());
        assert!(vault_dir.join("people/contact/jane.md").exists());
    }

    #[test]
    fn dry_run_reports_plan_without_moving_anything() {
        let temp = TempDir::new().unwrap();
        let vault_dir = temp.path();
        write_entry(vault_dir, "entities/contact", "jane.md", "contact");

        fs::write(
            vault_dir.join("vault.toml"),
            r#"
schema_version = 1
[default]
category = "knowledge"
tier = "working"
[kinds.contact]
category = "people"
identity = true
"#,
        )
        .unwrap();

        let summary = plan_reorg(vault_dir.to_str().unwrap(), true).unwrap();
        assert_eq!(summary.moved.len(), 1);
        // dry run: the file must not actually have moved.
        assert!(vault_dir.join("entities/contact/jane.md").exists());
        assert!(!vault_dir.join("people/contact/jane.md").exists());
    }

    #[test]
    fn leaves_already_correctly_placed_files_alone() {
        let temp = TempDir::new().unwrap();
        let vault_dir = temp.path();
        // No vault.toml -> zero-config default is a flat "insight/" folder, so a file
        // already sitting there needs no move.
        write_entry(vault_dir, "insight", "note.md", "insight");

        let summary = apply_reorg(vault_dir.to_str().unwrap()).unwrap();
        assert_eq!(summary.moved.len(), 0);
        assert_eq!(summary.unchanged, 1);
    }
}
