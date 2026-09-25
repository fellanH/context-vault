use std::sync::Arc;

use serde_json::{json, Value};

use crate::engine::VaultEngine;
use super::{tool_schemas, handlers_read, handlers_recall, handlers_write, handlers_ingest};

const TOOL_TIMEOUT_MS: u64 = 120_000; // 120 second timeout per tool call

pub(crate) async fn dispatch_tool_call(
    engine: &Arc<dyn VaultEngine>,
    id: Option<Value>,
    msg: &Value,
) -> Value {
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = tokio::time::timeout(
        tokio::time::Duration::from_millis(TOOL_TIMEOUT_MS),
        handle_tool(engine.clone(), name, arguments),
    )
    .await;

    let (content, is_error) = match result {
        Ok(Ok(text)) => {
            (vec![json!({ "type": "text", "text": text })], false)
        }
        Ok(Err(e)) => {
            (vec![json!({ "type": "text", "text": format!("Tool error: {e}") })], true)
        }
        Err(_) => {
            (vec![json!({ "type": "text", "text": format!("Tool timeout after {}ms", TOOL_TIMEOUT_MS) })], true)
        }
    };
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": content,
            "isError": is_error
        }
    })
}

pub(crate) fn reply_initialize(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "context-vault-rust-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    })
}

pub(crate) fn reply_tools_list(id: Option<Value>) -> Value {
    let mut tools = vec![];

    // 11 shipping tools with full schemas and annotations. clear_context (verified no-op —
    // just echoed its input back, touched no state) and publish_to_team (verified: every
    // call unconditionally errors, no working code path) were dropped rather than shipped
    // as tools that misrepresent what they do to an agent calling them.
    let tool_defs = vec![
        ("save_context", "Save knowledge to your vault. Creates a .md file and indexes it for search.", tool_schemas::save_context_schema()),
        ("get_context", "Search your knowledge vault. Returns entries ranked by relevance using hybrid full-text + semantic search.", tool_schemas::get_context_schema()),
        ("list_context", "List entries from your vault without search scoring. Use schema-based filtering by kind, category, tags.", tool_schemas::list_context_schema()),
        ("delete_context", "Delete an entry from your vault by its ID. Removes the file from disk and cleans up the search index.", tool_schemas::delete_context_schema()),
        ("ingest_url", "Fetch a URL, extract its readable content, and save it as a vault entry.", tool_schemas::ingest_url_schema()),
        ("ingest_project", "Scan a local project directory and register it as a project entity in the vault.", tool_schemas::ingest_project_schema()),
        ("context_status", "Show vault health: resolved config, file counts per kind, database size, and any issues.", tool_schemas::context_status_schema()),
        ("create_snapshot", "Pull all relevant vault entries matching a topic, deduplicate, and save as a structured context brief.", tool_schemas::create_snapshot_schema()),
        ("session_end", "Capture session summary and discoveries as vault entries. Call at end of session to preserve learnings.", tool_schemas::session_end_schema()),
        ("list_buckets", "List entries tagged as bucket entities in the vault.", tool_schemas::list_buckets_schema()),
        ("recall", "Search the vault using a raw signal (prompt text, error message, file path, or task context). Returns lightweight hints.", tool_schemas::recall_schema()),
    ];

    let annotations = tool_schemas::get_tool_annotations();
    let ann_map: std::collections::HashMap<_, _> = annotations
        .iter()
        .map(|a| (a.name, (a.read_only, a.destructive)))
        .collect();

    for (name, description, input_schema) in tool_defs {
        let (read_only, destructive) = ann_map.get(name).unwrap_or(&(false, false));

        let mut tool = json!({
            "name": name,
            "description": description,
            "inputSchema": input_schema
        });

        // Add annotations for MCP client UI hints
        if let Some(obj) = tool.as_object_mut() {
            obj.insert("_annotations".to_string(), json!({
                "readonly": read_only,
                "destructive": destructive
            }));
        }

        tools.push(tool);
    }

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": tools
        }
    })
}

pub(crate) fn reply_error(id: Option<Value>, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

async fn handle_tool(
    engine: Arc<dyn VaultEngine>,
    name: &str,
    args: Value,
) -> std::result::Result<String, String> {
    match name {
        "save_context" => handlers_write::handle_save_context(engine, &args).await,
        "get_context" => handlers_read::handle_get_context(engine, &args).await,
        "list_context" => handlers_read::handle_list_context(engine, &args).await,
        "delete_context" => handlers_write::handle_delete_context(engine, &args).await,
        "ingest_url" => handlers_ingest::handle_ingest_url(engine, &args).await,
        "ingest_project" => handlers_ingest::handle_ingest_project(engine, &args).await,
        "context_status" => handlers_read::handle_context_status(engine).await,
        "create_snapshot" => handlers_ingest::handle_create_snapshot(engine, &args).await,
        "session_end" => handlers_write::handle_session_end(engine, &args).await,
        "list_buckets" => handlers_read::handle_list_buckets(engine).await,
        "recall" => handlers_recall::handle_recall(engine, &args).await,
        _ => Err(format!("Unknown tool: {}", name)),
    }
}
