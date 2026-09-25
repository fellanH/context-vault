//! Schemas for session/snapshot tools, plus MCP tool annotations.

use serde_json::{json, Value};

pub fn create_snapshot_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "topic": { "type": "string", "description": "The topic or project name to snapshot" },
            "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional tag filters" },
            "buckets": { "type": "array", "items": { "type": "string" }, "description": "Filter by buckets" },
            "kinds": { "type": "array", "items": { "type": "string" }, "description": "Optional kind filters" },
            "identity_key": { "type": ["string", "null"], "description": "Deterministic key for the saved brief" }
        },
        "required": ["topic"]
    })
}

pub fn session_end_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": { "type": ["string", "null"], "description": "Summary of session accomplishments" },
            "discoveries": { "type": "array", "items": { "type": "object" }, "description": "Discrete discoveries to save" },
            "bucket": { "type": ["string", "null"], "description": "Optional bucket to scope the session" }
        },
        "required": []
    })
}

pub fn list_buckets_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "include_counts": { "type": "boolean", "description": "Include entry counts per bucket" }
        },
        "required": []
    })
}

pub fn recall_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "signal": { "type": "string", "description": "Raw text: prompt, error message, file path, or combined signal" },
            "signal_type": { "enum": ["prompt", "error", "file", "task"], "description": "Type of signal" },
            "bucket": { "type": ["string", "null"], "description": "Scope results to a project bucket" },
            "session_id": { "type": ["string", "null"], "description": "Session identifier for dedup" },
            "max_hints": { "type": "integer", "description": "Maximum hints to return (default 3)" }
        },
        "required": ["signal", "signal_type"]
    })
}

/// Tool annotations from TypeScript (readOnly, destructive hints for MCP UI).
pub struct ToolAnnotations {
    pub name: &'static str,
    pub read_only: bool,
    pub destructive: bool,
}

pub fn get_tool_annotations() -> Vec<ToolAnnotations> {
    vec![
        ToolAnnotations { name: "save_context", read_only: false, destructive: true },
        ToolAnnotations { name: "get_context", read_only: true, destructive: false },
        ToolAnnotations { name: "list_context", read_only: true, destructive: false },
        ToolAnnotations { name: "delete_context", read_only: false, destructive: true },
        ToolAnnotations { name: "ingest_url", read_only: false, destructive: false },
        ToolAnnotations { name: "ingest_project", read_only: false, destructive: false },
        ToolAnnotations { name: "context_status", read_only: true, destructive: false },
        ToolAnnotations { name: "create_snapshot", read_only: false, destructive: false },
        ToolAnnotations { name: "session_end", read_only: false, destructive: true },
        ToolAnnotations { name: "list_buckets", read_only: true, destructive: false },
        ToolAnnotations { name: "recall", read_only: true, destructive: false },
    ]
}
