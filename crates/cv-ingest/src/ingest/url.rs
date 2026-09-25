use anyhow::{anyhow, Result};
use serde_json::{Map, Value};
use std::time::Duration;
use crate::ingest::url_convert::extract_html_content;

#[derive(Debug, Clone)]
pub struct IngestResult {
    pub kind: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub meta: Map<String, Value>,
    pub source: String,
}

pub async fn ingest_url(
    url: &str,
    opts: IngestUrlOpts,
) -> Result<IngestResult> {
    let kind = opts.kind.unwrap_or_else(|| "reference".to_string());
    let tags = opts.tags.unwrap_or_default();
    let source = opts.source;
    let max_body_length = opts.max_body_length.unwrap_or(50000);
    let timeout_ms = opts.timeout_ms.unwrap_or(15000);

    // Extract domain from URL
    let parsed_url = reqwest::Url::parse(url)
        .map_err(|e| anyhow!("Invalid URL: {}", e))?;
    let domain = parsed_url
        .host_str()
        .ok_or_else(|| anyhow!("Invalid URL: no host"))?
        .to_string();

    // Create HTTP client with timeout
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_millis(timeout_ms))
        .user_agent("ContextVault/1.0 (+https://github.com/fellanH/context-vault)")
        .build()?;

    // Fetch URL
    let response = client.get(url).send().await
        .map_err(|e| anyhow!("Fetch failed: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "HTTP {}: {}",
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown")
        ));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let html = response.text().await?;

    let (title, body) = if content_type.contains("text/html")
        || content_type.contains("application/xhtml")
    {
        extract_html_content(&html, url)
    } else {
        (domain.clone(), html)
    };

    // Truncate if necessary
    let body = if body.len() > max_body_length {
        format!(
            "{}\n\n[Content truncated]",
            &body[..max_body_length]
        )
    } else {
        body
    };

    if body.trim().is_empty() {
        return Err(anyhow!("No readable content extracted from URL"));
    }

    let mut meta = Map::new();
    meta.insert("url".to_string(), Value::String(url.to_string()));
    meta.insert("domain".to_string(), Value::String(domain.clone()));
    meta.insert(
        "fetched_at".to_string(),
        Value::String(chrono::Utc::now().to_rfc3339()),
    );
    let ct = content_type.split(';').next().unwrap_or("text/html").trim();
    meta.insert("content_type".to_string(), Value::String(ct.to_string()));

    let mut result_tags = tags;
    result_tags.push("web-import".to_string());

    Ok(IngestResult {
        kind,
        title: if title.is_empty() { domain.clone() } else { title },
        body,
        tags: result_tags,
        meta,
        source: source.unwrap_or(domain),
    })
}

#[derive(Debug, Clone, Default)]
pub struct IngestUrlOpts {
    pub kind: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    pub max_body_length: Option<usize>,
    pub timeout_ms: Option<u64>,
}

#[cfg(test)]
mod url_tests {
    use super::*;

    #[tokio::test]
    async fn test_ingest_url_invalid() {
        let result = ingest_url("not a url", IngestUrlOpts::default()).await;
        assert!(result.is_err());
    }
}
