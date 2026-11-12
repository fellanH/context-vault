//! Markdown side of SqliteBackend::save: new entries get a new file, an id rewrites
//! the existing one in place, and a failed DB write rolls the file back.

use anyhow::{anyhow, Result};
use cv_engine::{SaveInput, VaultEngine};

use super::sqlite::SqliteBackend;

/// Writes the entry's markdown. With an id the entry is rewritten in place (same id,
/// created time and file); the previous content is returned so a DB failure can restore it.
pub(crate) async fn write_markdown(engine: &SqliteBackend, vault_dir: &str, input: &SaveInput) -> Result<(cv_vault::WriteResult, Option<Vec<u8>>)> {
    let Some(id) = &input.id else {
        return Ok((cv_vault::write_entry(vault_dir, input)?, None));
    };
    let existing = engine.get(id).await?.ok_or_else(|| anyhow!("Entry not found: {id}"))?;
    let path = existing.file_path.ok_or_else(|| anyhow!("Entry {id} has no markdown file to update"))?;
    let previous = std::fs::read(&path).ok();
    Ok((cv_vault::rewrite_entry(vault_dir, input, id, &existing.created_at, &path)?, previous))
}

/// Rolls back a markdown write: restore an update's previous content, remove a new file.
pub(crate) fn restore_file(path: &str, previous: Option<Vec<u8>>) -> std::io::Result<()> {
    match previous {
        Some(bytes) => std::fs::write(path, bytes),
        None => std::fs::remove_file(path),
    }
}
