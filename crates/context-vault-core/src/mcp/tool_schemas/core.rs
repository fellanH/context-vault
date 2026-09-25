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
            "source": { "type": ["string", "null"], "description": "Where this knowledge came from" },
            "identity_key": { "type": ["string", "null"], "description": "Required when kind is an entity kind" },
            "expires_at": { "type": ["string", "null"], "description": "ISO date marking TTL expiry" },
            "supersedes": { "type": "array", "items": { "type": "string" }, "description": "Array of entry IDs this entry supersedes" },
            "related_to": { "type": "array", "items": { "type": "string" }, "description": "Array of related entry IDs" },
            "dry_run": { "type": "boolean", "description": "Check whether similar entries already exist, without saving" },
            "similarity_threshold": { "type": "number", "description": "Cosine similarity threshold (0–1)" },
            "tier": { "enum": ["ephemeral", "working", "durable"], "description": "Memory tier governing lifecycle" },
            "conflict_resolution": { "enum": ["suggest", "off"], "description": "Conflict resolution mode" }
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
            "identity_key": { "type": ["string", "null"], "description": "For entity lookup: exact match" },
            "tags": { "type": "array", "items": { "type": "string" }, "description": "Filter by tags" },
            "buckets": { "type": "array", "items": { "type": "string" }, "description": "Filter by project buckets" },
            "since": { "type": ["string", "null"], "description": "Return entries created after this date" },
            "until": { "type": ["string", "null"], "description": "Return entries created before this date" },
            "limit": { "type": "integer", "description": "Max results to return (default 10)" },
            "include_superseded": { "type": "boolean", "description": "Include superseded entries" },
            "detect_conflicts": { "type": "boolean", "description": "Compare results for conflicts" },
            "max_tokens": { "type": "integer", "description": "Limit output to this token budget" },
            "pivot_count": { "type": "integer", "description": "Number of top results with full body" },
            "include_ephemeral": { "type": "boolean", "description": "Include ephemeral tier entries" },
            "scope": { "enum": ["hot", "events", "all"], "description": "Index scope" },
            "body_limit": { "type": "integer", "description": "Max characters of body per entry" }
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
            "tags": { "type": "array", "items": { "type": "string" }, "description": "Filter by tags" },
            "buckets": { "type": "array", "items": { "type": "string" }, "description": "Filter by buckets" },
            "since": { "type": ["string", "null"], "description": "Return entries created after this date" },
            "until": { "type": ["string", "null"], "description": "Return entries created before this date" },
            "limit": { "type": "integer", "description": "Max results to return (default 10)" },
            "include_superseded": { "type": "boolean", "description": "Include superseded entries" },
            "include_ephemeral": { "type": "boolean", "description": "Include ephemeral entries" },
            "include_unindexed": { "type": "boolean", "description": "Include unindexed entries" }
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
