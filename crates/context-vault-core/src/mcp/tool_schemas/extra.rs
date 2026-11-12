//! Schemas for session/snapshot tools, plus MCP tool annotations.

use serde_json::{json, Value};

pub fn create_snapshot_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "topic": { "type": "string", "description": "The topic or project name to snapshot" }
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
        },
        "required": []
    })
}

pub fn recall_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "signal": { "type": "string", "description": "Raw text: prompt, error message, file path, or combined signal" },
            "bucket": { "type": ["string", "null"], "description": "Scope results to a project bucket" },
            "max_hints": { "type": "integer", "description": "Maximum hints to return (default 3)" }
        },
        "required": ["signal"]
    })
}

/// Tool annotations from TypeScript (readOnly, destructive hints for MCP UI).
pub struct ToolAnnotations {
    pub name: &'static str,
    pub read_only: bool,
    pub destructive: bool,
}

pub fn context_for_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "situation": { "type": "string", "description": "Situation name, lowercase-kebab (fresh, post-compaction, before-deploy, before-design, debugging, onboarding)" },
            "cursor": { "type": ["string", "null"], "description": "next_cursor from the previous page" }
        },
        "required": ["situation"]
    })
}

pub fn list_situations_schema() -> Value {
    json!({ "type": "object", "properties": {}, "required": [] })
}

pub fn list_vaults_schema() -> Value {
    json!({ "type": "object", "properties": {}, "required": [] })
}

pub fn vault_map_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "refresh": { "type": "boolean", "description": "Regenerate now instead of reading the cached map (at most once a minute)" }
        },
        "required": []
    })
}

pub fn vault_schema_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": { "type": ["string", "null"], "description": "One kind to describe (its effective rule, falling back to [default]); omit for all kinds" }
        },
        "required": []
    })
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
        ToolAnnotations { name: "list_vaults", read_only: true, destructive: false },
        ToolAnnotations { name: "create_snapshot", read_only: false, destructive: false },
        ToolAnnotations { name: "session_end", read_only: false, destructive: true },
        ToolAnnotations { name: "list_buckets", read_only: true, destructive: false },
        ToolAnnotations { name: "recall", read_only: true, destructive: false },
        ToolAnnotations { name: "context_for", read_only: true, destructive: false },
        ToolAnnotations { name: "list_situations", read_only: true, destructive: false },
        ToolAnnotations { name: "vault_map", read_only: true, destructive: false },
        ToolAnnotations { name: "vault_schema", read_only: true, destructive: false },
    ]
}
