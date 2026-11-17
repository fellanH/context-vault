//! Read commands: save, get, search, list, recall.

use std::sync::Arc;

use anyhow::Result;
use context_vault_core::engine::{ListOptions, SaveInput, ScoredEntry, SearchOptions, VaultEngine};

use crate::engine_cmd;

#[allow(clippy::too_many_arguments)] // mirrors the CLI flag surface 1:1
pub async fn run_save(
    kind: String,
    title: String,
    body: Option<String>,
    tags: Option<String>,
    tier: Option<String>,
    source: Option<String>,
    meta: Option<String>,
    identity_key: Option<String>,
) -> Result<()> {
    let engine = engine_cmd::build_engine().await?;
    let tags_vec = tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect());
    let meta_obj = meta.map(|m| serde_json::from_str(&m)).transpose()?;

    let input = SaveInput {
        kind,
        title: Some(title),
        body: body.unwrap_or_default(),
        tags: tags_vec,
        tier: tier.or(Some("working".to_string())),
        source: source.or(Some("cli".to_string())),
        meta: meta_obj,
        identity_key,
        ..Default::default()
    };

    let output = engine.save(input).await?;
    println!("{}", output.id);
    Ok(())
}

pub async fn run_get(id: &str) -> Result<()> {
    let engine = engine_cmd::build_engine().await?;
    if let Some(entry) = engine.get(id).await? {
        println!("{}", serde_json::to_string_pretty(&entry)?);
    } else {
        eprintln!("Entry not found: {}", id);
        std::process::exit(1);
    }
    Ok(())
}

pub async fn run_search(query: &str, limit: Option<usize>, format: &str) -> Result<()> {
    let engine = engine_cmd::build_engine().await?;
    let opts = SearchOptions {
        limit: limit.unwrap_or(50),
        ..Default::default()
    };
    let results = engine.hybrid_search(query, opts).await?;

    match format {
        "json" => println!("{}", serde_json::to_string_pretty(&results)?),
        "text" => print_text(query, &results),
        other => return Err(anyhow::anyhow!("unknown --format '{other}', expected 'text' or 'json'")),
    }
    Ok(())
}

/// One line per hit; a miss says so on stderr, so an empty answer is never silent (OPS-288).
fn print_text(query: &str, results: &[ScoredEntry]) {
    if results.is_empty() {
        eprintln!("No results for \"{query}\". Try a word that is in the entry, such as a name or a term from its title.");
    }
    for result in results {
        println!("{}  {:.4}  {}", result.entry.id, result.score, result.entry.title.as_deref().unwrap_or(""));
        println!("    {}", snippet(&result.entry.body));
    }
}

/// The body on one line, cut at about 120 characters, so the answer shows under its title.
fn snippet(body: &str) -> String {
    let line = body.split_whitespace().collect::<Vec<_>>().join(" ");
    match line.char_indices().nth(120) {
        Some((cut, _)) => format!("{}…", &line[..cut]),
        None => line,
    }
}

pub async fn run_recall(
    signal: &str,
    limit: Option<usize>,
    bucket: Option<String>,
    global: bool,
) -> Result<()> {
    // Recall is the passive-hint surface, so it scopes to the current project by
    // default (unlike `search`, the explicit whole-vault explorer). `--bucket`
    // overrides the cwd-derived scope; `--global` disables scoping.
    let scope = if global {
        None
    } else {
        bucket.or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|d| crate::cmd_recall_hint::project_from_cwd(&d.to_string_lossy()))
        })
    };
    let engine: Arc<dyn VaultEngine> = engine_cmd::build_engine().await?;
    match context_vault_core::mcp::recall(engine, signal, scope.as_deref(), limit.unwrap_or(3)).await
    {
        Ok(hints) => println!("{hints}"),
        Err(e) => return Err(anyhow::anyhow!("recall failed: {e}")),
    }
    Ok(())
}

pub async fn run_list(
    kind: Option<String>,
    tier: Option<String>,
    limit: Option<usize>,
) -> Result<()> {
    let engine = engine_cmd::build_engine().await?;
    let opts = ListOptions {
        kind,
        tier,
        limit: limit.unwrap_or(50),
        ..Default::default()
    };
    let entries = engine.list(opts).await?;
    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}
