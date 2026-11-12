use std::io::{self, BufRead, BufReader, Write};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::backends::{FastembedEmbedder, NoOpEmbedder, SqliteBackend};
use crate::engine::{Embedder, VaultEngine};
use crate::paths::{resolve_db_path, resolve_model_cache_dir, resolve_vault_dir};
use super::tools;

pub async fn run() -> Result<()> {
    let engine = open_engine().await?;

    engine.init().await?;

    // No reindex at startup. Saves through the engine index themselves; only files
    // edited outside it need `context-vault reindex` (incremental). A startup walk
    // used to cost every session ~48 s and 24 CPU-s on an 85k-file vault
    // (measured 2026-09-25), for nothing when files had not changed.

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = io::stdout().lock();

    while process_one_message(&mut reader, &mut stdout, &engine).await? {}

    Ok(())
}

async fn process_one_message<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    engine: &Arc<dyn VaultEngine>,
) -> Result<bool> {
    let body = match read_mcp_message(reader)? {
        Some(b) => b,
        None => return Ok(false),
    };

    let msg: Value =
        serde_json::from_slice(&body).map_err(|e| anyhow!("invalid JSON-RPC: {e}"))?;
    let id = msg.get("id").cloned();
    let method = msg
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or_default();

    if method.starts_with("notifications/") {
        return Ok(true);
    }

    let reply = dispatch_method(engine, method, id, &msg).await;
    write_mcp_message(writer, &reply)?;
    Ok(true)
}

/// v5 when `vaults.toml` lists vaults (written by `migrate`, card 38), else v4 as before.
async fn open_engine() -> Result<Arc<dyn VaultEngine>> {
    if let Some(v5) = cv_index::engine::V5Backend::from_registry()? {
        eprintln!("[context-vault] v5 engine (vaults.toml)");
        return Ok(Arc::new(v5));
    }
    let db_path = resolve_db_path()?;
    let vault_dir = resolve_vault_dir()?;
    let embedder = init_embedder().await?;
    Ok(Arc::new(SqliteBackend::new(&db_path, &vault_dir, embedder)?))
}

async fn init_embedder() -> Result<Arc<dyn Embedder>> {
    let cache_dir = std::path::PathBuf::from(resolve_model_cache_dir()?);
    let embedder: Arc<dyn Embedder> =
        match tokio::task::spawn_blocking(move || FastembedEmbedder::new(cache_dir)).await? {
            Ok(e) => {
                eprintln!("[context-vault] Embedding model loaded");
                Arc::new(e)
            }
            Err(e) => {
                eprintln!("[context-vault] Failed to load embedding model: {e}");
                eprintln!("[context-vault] Semantic search disabled. Full-text search still works.");
                Arc::new(NoOpEmbedder)
            }
        };
    Ok(embedder)
}

async fn dispatch_method(
    engine: &Arc<dyn VaultEngine>,
    method: &str,
    id: Option<Value>,
    msg: &Value,
) -> Value {
    match method {
        "initialize" => tools::reply_initialize(id),
        "tools/list" => tools::reply_tools_list(id),
        "tools/call" => tools::dispatch_tool_call(engine, id, msg).await,
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        _ => tools::reply_error(id, -32601, &format!("method not implemented: {method}")),
    }
}

// MCP stdio transport is newline-delimited JSON (one message per line, no
// embedded newlines) — NOT LSP-style Content-Length framing. A prior version
// of this file used Content-Length headers, which caused every client
// (including Claude Code) to hang forever on connect: the server sat in
// read_headers waiting for a header line the client never sends.
fn read_mcp_message<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        return Ok(Some(trimmed.as_bytes().to_vec()));
    }
}

fn write_mcp_message<W: Write>(w: &mut W, v: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(v).map_err(io::Error::other)?;
    w.write_all(&body)?;
    w.write_all(b"\n")?;
    w.flush()
}
