//! Saves and deletes on knowledge vaults: write the file, then make sure the index has it.
use super::{V5Backend, Vault};
use crate::UpdateOptions;
use anyhow::{anyhow, bail, Result};
use cv_core::VaultType;
use cv_engine::{Entry, SaveInput, SaveOutput};

pub(super) fn save(engine: &V5Backend, mut input: SaveInput) -> Result<SaveOutput> {
    let Some(id) = input.id.clone() else {
        let vault = engine.knowledge_default()?;
        drop_self_supersede(&mut input, None);
        if let Some(data) = exhaust_target(engine, vault, &input.kind) {
            return append_exhaust(vault, data, &input);
        }
        let written = cv_vault::capture::write_entry(&vault.index.root().to_string_lossy(), &input)?;
        index_written(vault, &written.file_path)?;
        return Ok(SaveOutput { id: written.id });
    };
    let (vault, existing) = engine.find(&id)?.ok_or_else(|| anyhow!("Entry not found: {id}"))?;
    if vault.entry.kind != VaultType::Knowledge {
        bail!("`{id}` is in data vault `{}`: append a record that supersedes it instead", vault.name);
    }
    drop_self_supersede(&mut input, Some(&id));
    keep_relations(&mut input, &existing);
    let path = existing.file_path.clone().ok_or_else(|| anyhow!("Entry {id} has no markdown file to update"))?;
    let dir = vault.index.root().to_string_lossy().into_owned();
    let written = cv_vault::capture::rewrite_entry(&dir, &input, &id, &existing.created_at, &path)?;
    index_written(vault, &written.file_path)?;
    Ok(SaveOutput { id })
}

pub(super) fn delete(engine: &V5Backend, id: &str) -> Result<()> {
    let (vault, entry) = engine.find(id)?.ok_or_else(|| anyhow!("Entry not found: {id}"))?;
    if vault.entry.kind != VaultType::Knowledge {
        bail!("`{id}` is in data vault `{}`: append a record that supersedes it instead", vault.name);
    }
    let path = entry.file_path.ok_or_else(|| anyhow!("Entry {id} has no markdown file"))?;
    std::fs::remove_file(&path)?;
    index_written(vault, &path)
}

/// Machine records (vault.toml category `events`, and `rule-candidate`) belong in the data
/// vault: the knowledge index leaves events/ out (migrate moved them there), so a file
/// written there would never be found. Without a data vault, they stay files as in v4.
fn exhaust_target<'a>(engine: &'a V5Backend, main: &Vault, kind: &str) -> Option<&'a Vault> {
    let exhaust = main.taxonomy.category_for(kind) == "events" || kind == "rule-candidate";
    let data = engine.vaults.iter().filter(|v| v.entry.kind == VaultType::Data);
    exhaust.then(|| data.clone().find(|v| v.name == "exhaust").or_else(|| data.clone().next())).flatten()
}

/// One record appended to the data vault's stream for its kind, after the same vault.toml
/// checks a knowledge save gets, then indexed so `get` finds it at once.
fn append_exhaust(main: &Vault, data: &Vault, input: &SaveInput) -> Result<SaveOutput> {
    let taxonomy = cv_vault::VaultTaxonomy::load_strict(&main.index.root().to_string_lossy())?;
    cv_vault::check_save(&taxonomy, input)?;
    let mut record = cv_core::Record::new(&input.kind, &input.body);
    record.title = input.title.clone();
    record.source = input.source.clone();
    record.tags = input.tags.clone().unwrap_or_default();
    record.supersedes = input.supersedes.clone().unwrap_or_default();
    if let Some(serde_json::Value::Object(meta)) = &input.meta {
        record.meta = meta.clone();
    }
    cv_core::append(data.index.root(), &input.kind, &record)?;
    for _ in 0..50 {
        let report = data.index.update(&UpdateOptions { budget: None, ..Default::default() })?;
        if !report.busy {
            return Ok(SaveOutput { id: record.id });
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    eprintln!("[context-vault] index writer busy for 5 s; {} is saved and the next catch-up indexes it", record.id);
    Ok(SaveOutput { id: record.id })
}

/// v4 kept these relations in its database, so an update that omitted them kept them.
/// In v5 they live only in the file: an update that doesn't name them carries them over.
fn keep_relations(input: &mut SaveInput, existing: &Entry) {
    input.supersedes = input.supersedes.take().or_else(|| existing.supersedes.clone());
    input.related_to = input.related_to.take().or_else(|| existing.related_to.clone());
    input.expires_at = input.expires_at.take().or_else(|| existing.expires_at.clone());
    input.identity_key = input.identity_key.take().or_else(|| existing.identity_key.clone());
    input.tier = input.tier.take().or_else(|| Some(existing.tier.clone()));
}

/// A record never supersedes itself (v4 skipped it too); empty lists are dropped.
fn drop_self_supersede(input: &mut SaveInput, id: Option<&str>) {
    if let Some(list) = input.supersedes.as_mut() {
        list.retain(|old| Some(old.as_str()) != id && !old.is_empty());
        if list.is_empty() {
            input.supersedes = None;
        }
    }
}

/// The file is indexed (or forgotten, if deleted) before the budgeted catch-up runs, so the
/// next call sees it. A writer held by another process is waited for, up to 5 s.
fn index_written(vault: &Vault, file: &str) -> Result<()> {
    let options = UpdateOptions { paths: vec![std::path::PathBuf::from(file)], ..Default::default() };
    for _ in 0..50 {
        if !vault.index.update(&options)?.busy {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // The write itself succeeded; an error here would make a retrying caller save it twice.
    eprintln!("[context-vault] index writer busy for 5 s; {file} is saved and the next catch-up indexes it");
    Ok(())
}

#[cfg(test)]
#[path = "engine_write_tests.rs"]
mod tests;
