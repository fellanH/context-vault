//! Ingest handlers: ingest_url, ingest_project, create_snapshot.
//! These handle specialized data ingestion workflows.

use std::sync::Arc;
use serde_json::Value;
use crate::engine::{VaultEngine, SaveInput};
use crate::ingest::url::{ingest_url, IngestUrlOpts};

/// Implements ingest_url: fetch a URL, extract readable content, and save as a vault entry.
/// Routes through the same `cv_ingest::ingest::url::ingest_url` the CLI's `ingest` command
/// uses — this used to be a hand-written stub that never fetched anything; now it shares
/// the real fetch + readability-extraction implementation.
pub async fn handle_ingest_url(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("Required: url")?;

    if url.len() > 2048 {
        return Err("url must be under 2048 chars".to_string());
    }

    let kind = args
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("reference")
        .to_string();

    let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
    });

    let opts = IngestUrlOpts {
        kind: Some(kind),
        tags,
        ..Default::default()
    };

    let result = ingest_url(url, opts).await.map_err(|e| e.to_string())?;

    let input = SaveInput {
        kind: result.kind,
        title: Some(result.title.clone()),
        body: result.body,
        tags: Some(result.tags),
        source: Some(result.source),
        meta: Some(serde_json::json!(result.meta)),
        ..Default::default()
    };

    let output = engine.save(input).await.map_err(|e| e.to_string())?;

    Ok(format!(
        "✓ Ingested URL → `{}`\n  id: {}\n  source: {}",
        result.title, output.id, url
    ))
}

/// Implements ingest_project: scan a local project directory (package.json, README,
/// git remote/last-commit, tech-stack detection, CLAUDE.md presence) and register it
/// as a project entity. Routes through the same `cv_ingest::ingest::project::ingest_project`
/// the CLI's `ingest-project` command uses — this used to be a hand-written stub that
/// never scanned anything; now it shares the real scanner.
pub async fn handle_ingest_project(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Required: path")?;

    let tags = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
    });

    let pillar = args
        .get("pillar")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let result = crate::ingest::project::ingest_project(path, tags, pillar)
        .map_err(|e| e.to_string())?;

    let input = SaveInput {
        kind: "project".to_string(),
        title: Some(result.title.clone()),
        body: result.body,
        tags: Some(result.tags),
        identity_key: Some(result.identity_key),
        ..Default::default()
    };

    let output = engine.save(input).await.map_err(|e| e.to_string())?;

    Ok(format!(
        "✓ Ingested project `{}`\n  id: {}\n  path: {}",
        result.title, output.id, path
    ))
}

/// Implements create_snapshot: gather entries by topic and save as a brief.
pub async fn handle_create_snapshot(
    engine: Arc<dyn VaultEngine>,
    args: &Value,
) -> Result<String, String> {
    let topic = args
        .get("topic")
        .and_then(|v| v.as_str())
        .ok_or("Required: topic")?;

    // Query entries matching the topic
    let opts = crate::engine::SearchOptions {
        limit: 40,
        offset: 0,
        ..Default::default()
    };

    let results = engine
        .hybrid_search(topic, opts)
        .await
        .map_err(|e| e.to_string())?;

    if results.is_empty() {
        return Ok(format!("No entries found for topic: {}", topic));
    }

    // Format gathered entries as markdown
    let mut body = format!("# {} — Context Brief\n\n", topic);
    body.push_str(&format!(
        "*Gathered from {} vault {}. Synthesize the content below.*\n\n---\n\n",
        results.len(),
        if results.len() == 1 { "entry" } else { "entries" }
    ));

    for (i, scored) in results.iter().enumerate() {
        let e = &scored.entry;
        let title = e.title.as_deref().unwrap_or("(untitled)");
        body.push_str(&format!("## {}. [{}] {}\n\n", i + 1, e.kind, title));
        body.push_str(&format!("**ID:** `{}`\n\n", e.id));
        body.push_str(&format!("**Created:** {}\n\n", e.created_at));
        body.push_str(&e.body);
        body.push_str("\n\n---\n\n");
    }

    // Save the brief as a new entry
    let identity_key = slugify_topic(topic);
    let input = SaveInput {
        kind: "brief".to_string(),
        title: Some(format!("{} — Brief", topic)),
        body,
        identity_key: Some(identity_key.clone()),
        tags: Some(vec![format!("snapshot:{}", topic)]),
        tier: Some("working".to_string()),
        ..Default::default()
    };

    let output = engine.save(input).await.map_err(|e| e.to_string())?;

    Ok(format!(
        "✓ Created snapshot\n  id: {}\n  entries: {}\n  identity_key: {}",
        output.id,
        results.len(),
        identity_key
    ))
}

/// Slugify a topic string for use as identity_key.
fn slugify_topic(topic: &str) -> String {
    topic
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(120)
        .collect()
}
