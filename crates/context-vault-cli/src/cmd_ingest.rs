//! Ingest commands: ingest URL, ingest project directory.

use anyhow::Result;
use context_vault_core::engine::{SaveInput, VaultEngine};
use context_vault_core::ingest::project::ingest_project;
use context_vault_core::ingest::url::{ingest_url, IngestUrlOpts};

use crate::engine_cmd;

pub async fn run_ingest(url: &str) -> Result<()> {
    let engine = engine_cmd::build_engine().await?;
    let opts = IngestUrlOpts {
        kind: Some("reference".to_string()),
        ..Default::default()
    };

    let result = ingest_url(url, opts).await?;
    let input = SaveInput {
        kind: result.kind,
        title: Some(result.title),
        body: result.body,
        tags: Some(result.tags),
        source: Some(result.source),
        meta: Some(serde_json::json!(result.meta)),
        ..Default::default()
    };

    let output = engine.save(input).await?;
    println!("ingested from URL: {}", output.id);
    Ok(())
}

pub async fn run_ingest_project(path: &str) -> Result<()> {
    let engine = engine_cmd::build_engine().await?;
    let result = ingest_project(path, None, None)?;

    let input = SaveInput {
        kind: "project".to_string(),
        title: Some(result.title.clone()),
        body: result.body,
        tags: Some(result.tags),
        identity_key: Some(result.identity_key),
        ..Default::default()
    };

    let output = engine.save(input).await?;
    println!("ingested project: {} — {}", result.title, output.id);
    Ok(())
}
