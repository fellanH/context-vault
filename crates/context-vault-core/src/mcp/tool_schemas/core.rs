//! Schemas for the core read/write vault tools.

use serde_json::{json, Value};

pub fn save_context_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": ["string", "null"], "description": "Entry ID to update. When provided, updates existing entry instead of creating new." },
            "kind": { "type": "string", "description": "Entry kind (e.g. 'insight', 'decision', 'pattern'). Required when creating a new entry." },
            "title": { "type": ["string", "null"], "description": "Entry title (optional when kind is insight)" },
            "body": { "type": "string", "description": "Main content. Required when creating a new entry." },
            "tags": { "type": "array", "items": { "type": "string" }, "description": "Tags used to categorize and aid search" },
            "meta": { "type": "object", "description": "Additional structured metadata" },
            "situations": { "type": ["array", "null"], "items": { "type": "string" }, "description": "When this entry matters, e.g. fresh, post-compaction, before-deploy, before-design, debugging, onboarding. Lowercase-kebab, max 6, 32 chars each. context_for loads them by situation. null or [] clears (on update)." },
            "situation_order": { "type": ["integer", "null"], "description": "Load order within a situation, 0..100000, lowest first (default 100). Alone, it re-orders an existing entry." },
            "source": { "type": ["string", "null"], "description": "Where this knowledge came from" },
            "identity_key": { "type": ["string", "null"], "description": "Required when kind is an entity kind" },
            "expires_at": { "type": ["string", "null"], "description": "ISO date marking TTL expiry" },
            "supersedes": { "type": "array", "items": { "type": "string" }, "description": "Array of entry IDs this entry supersedes" },
            "related_to": { "type": "array", "items": { "type": "string" }, "description": "Array of related entry IDs" },
            "tier": { "enum": ["ephemeral", "working", "durable"], "description": "Memory tier governing lifecycle" }
        },
        "required": []
    })
}

pub fn get_context_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": { "type": ["string", "null"], "description": "Search query (natural language or keywords)" },
            "kind": { "type": ["string", "null"], "description": "Filter by kind" },
            "category": { "type": ["string", "null"], "description": "Filter by category — vault-defined via vault.toml, not a fixed set" },
            "tags": { "type": "array", "items": { "type": "string" }, "description": "Filter by tags (an entry must carry all of them)" },
            "since": { "type": ["string", "null"], "description": "Return entries created after this date" },
            "until": { "type": ["string", "null"], "description": "Return entries created before this date" },
            "limit": { "type": "integer", "description": "Max results to return (default 10)" },
            "include_superseded": { "type": "boolean", "description": "Include superseded and expired entries (left out by default)" },
            "max_tokens": { "type": "integer", "description": "Approximate output budget in tokens (about 4 characters each); results past it are left out" },
            "body_limit": { "type": "integer", "description": "Max characters of the body line shown per entry" }
        },
        "required": []
    })
}

pub fn list_context_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": { "type": ["string", "null"], "description": "Filter by kind" },
            "category": { "type": ["string", "null"], "description": "Filter by category — vault-defined via vault.toml, not a fixed set" },
            "limit": { "type": "integer", "description": "Max results to return (default 10)" }
        },
        "required": []
    })
}

pub fn delete_context_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "description": "The entry ID to delete" }
        },
        "required": ["id"]
    })
}

pub fn ingest_url_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "url": { "type": "string", "description": "The URL to fetch and save" },
            "kind": { "type": ["string", "null"], "description": "Entry kind (default: reference)" },
            "tags": { "type": "array", "items": { "type": "string" }, "description": "Tags for the entry" }
        },
        "required": ["url"]
    })
}

pub fn ingest_project_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "Absolute path to project directory" },
            "tags": { "type": "array", "items": { "type": "string" }, "description": "Additional tags" },
            "pillar": { "type": ["string", "null"], "description": "Parent pillar/domain name" }
        },
        "required": ["path"]
    })
}

pub fn context_status_schema() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "required": []
    })
}
