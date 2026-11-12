//! Knowledge files that stay in main but carry no id: the knowledge index skips them, so
//! they would vanish from search. Each gets its v4 id (else one derived from its path)
//! written into its frontmatter. The only change migrate makes to main; git-revertable.
//! Written atomically (temp file + rename) and only into real frontmatter.
use super::{is_exhaust, stable_id, MigrateOptions, MigrateReport};
use anyhow::Result;
use std::path::Path;

const MAX_NAMED: usize = 200;

pub(crate) fn persist_ids(options: &MigrateOptions, report: &mut MigrateReport) -> Result<()> {
    let walker = walkdir::WalkDir::new(&options.main).sort_by_file_name().into_iter();
    for entry in walker.filter_entry(|e| e.path().strip_prefix(&options.main).is_ok_and(|rel| !skipped(rel))) {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let key = path.strip_prefix(&options.main)?.to_string_lossy().into_owned();
        let (id, legacy) = match options.legacy_ids.get(&key) {
            Some(id) => (id.clone(), true),
            None => (stable_id(&format!("migrate:{key}")), false),
        };
        if write_id_if_missing(path, &id)? {
            report.knowledge_ids_written += 1;
            report.knowledge_ids_minted += u64::from(!legacy);
            if report.knowledge_id_files.len() < MAX_NAMED {
                report.knowledge_id_files.push(key);
            }
        }
    }
    Ok(())
}

/// What the knowledge index never reads: exhaust (events/, rule-candidate) and hidden or
/// tool folders (.git, .obsidian, node_modules). Nothing there is edited.
fn skipped(rel: &Path) -> bool {
    let hidden = rel.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        name.starts_with('.') || name == "node_modules"
    });
    hidden || rel.starts_with("events") || is_exhaust(rel)
}

/// Inserts `id:` as the first frontmatter line. A file without a closed frontmatter block,
/// or with any `id` key (whatever its type), is left alone.
fn write_id_if_missing(path: &Path, id: &str) -> Result<bool> {
    let raw = std::fs::read_to_string(path)?;
    let newline = if raw.starts_with("---\r\n") { "\r\n" } else if raw.starts_with("---\n") { "\n" } else { return Ok(false) };
    let rest = &raw[3 + newline.len()..];
    let closed = rest.starts_with("---") || rest.contains(&format!("{newline}---"));
    let (frontmatter, _) = cv_vault::parse_frontmatter(&raw);
    if !closed || frontmatter.contains_key("id") {
        return Ok(false);
    }
    replace_atomically(path, &format!("---{newline}id: {id}{newline}{rest}"))?;
    Ok(true)
}

/// A crash or a full disk leaves either the old file or the new one, never a torn note.
fn replace_atomically(path: &Path, text: &str) -> Result<()> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.cv-migrate.tmp"));
    let written = std::fs::write(&tmp, text)
        .and_then(|_| std::fs::set_permissions(&tmp, std::fs::metadata(path)?.permissions()))
        .and_then(|_| std::fs::rename(&tmp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(written?)
}
