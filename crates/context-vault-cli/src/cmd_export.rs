//! `export --kind <k> [--format csv|json]`: one kind's entries from the markdown files.

use anyhow::{bail, Result};
use context_vault_core::paths::{resolve_db_path, resolve_vault_dir};
use cv_vault::export::{collect, to_csv, to_json, ExportRow};
use cv_vault::VaultTaxonomy;
use rusqlite::{Connection, OpenFlags, OptionalExtension};

pub fn run_export(kind: &str, format: &str) -> Result<()> {
    if format != "csv" && format != "json" {
        bail!("--format must be csv or json");
    }
    let vault_dir = resolve_vault_dir()?;
    let taxonomy = VaultTaxonomy::load(&vault_dir);
    let (declared, mut rows) = collect(&vault_dir, &taxonomy, kind)?;
    fill_missing_ids(&mut rows)?;

    if format == "csv" {
        print!("{}", to_csv(&declared, &rows));
    } else {
        println!("{}", serde_json::to_string_pretty(&to_json(&declared, &rows))?);
    }
    eprintln!("{} {kind} entries exported", rows.len());
    Ok(())
}

/// Older files carry no `id` in frontmatter; the index knows theirs (read-only).
fn fill_missing_ids(rows: &mut [ExportRow]) -> Result<()> {
    if rows.iter().all(|r| r.id.is_some()) {
        return Ok(());
    }
    let Ok(conn) = Connection::open_with_flags(resolve_db_path()?, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return Ok(());
    };
    let mut stmt = conn.prepare("SELECT id FROM vault WHERE file_path = ?1")?;
    for r in rows.iter_mut().filter(|r| r.id.is_none()) {
        r.id = stmt.query_row([&r.file_path], |row| row.get(0)).optional()?;
    }
    Ok(())
}
