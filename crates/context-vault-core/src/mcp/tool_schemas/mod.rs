//! JSON schemas for MCP tool inputs.
//! These are shared across tools and follow the TypeScript tool definitions exactly.

mod core;
mod extra;

pub use core::*;
pub use extra::*;

#[cfg(test)]
mod honesty_tests {
    //! Every property a tool schema advertises must be read by that tool's handler
    //! (card 18): a param the handler ignores misleads the agent calling it.

    use serde_json::Value;


    /// Source of `pub async fn handle_<tool>` up to the next top-level fn.
    fn handler_body(src: &'static str, tool: &str) -> &'static str {
        let start = src
            .find(&format!("pub async fn handle_{tool}("))
            .unwrap_or_else(|| panic!("no handler for {tool}"));
        let rest = &src[start..];
        let end = rest[1..].find("\npub ").map(|i| i + 1).unwrap_or(rest.len());
        &rest[..end]
    }

    #[test]
    fn every_schema_property_is_read_by_its_handler() {
        let read = include_str!("../handlers_read.rs");
        let write = include_str!("../handlers_write.rs");
        let ingest = include_str!("../handlers_ingest.rs");
        let recall = include_str!("../handlers_recall.rs");
        let situations = include_str!("../handlers_situations.rs");
        let schema = include_str!("../handlers_schema.rs");
        let tools: Vec<(&str, Value, &'static str)> = vec![
            ("save_context", super::save_context_schema(), write),
            ("get_context", super::get_context_schema(), read),
            ("list_context", super::list_context_schema(), read),
            ("delete_context", super::delete_context_schema(), write),
            ("ingest_url", super::ingest_url_schema(), ingest),
            ("ingest_project", super::ingest_project_schema(), ingest),
            ("context_status", super::context_status_schema(), read),
            ("create_snapshot", super::create_snapshot_schema(), ingest),
            ("session_end", super::session_end_schema(), write),
            ("list_buckets", super::list_buckets_schema(), read),
            ("recall", super::recall_schema(), recall),
            ("context_for", super::context_for_schema(), situations),
            ("list_situations", super::list_situations_schema(), situations),
            ("list_vaults", super::list_vaults_schema(), situations),
            ("vault_map", super::vault_map_schema(), situations),
            ("vault_schema", super::vault_schema_schema(), schema),
        ];
        let mut unread = vec![];
        for (tool, schema, src) in tools {
            let body = handler_body(src, tool);
            for prop in schema["properties"].as_object().unwrap().keys() {
                if !body.contains(&format!("\"{prop}\"")) {
                    unread.push(format!("{tool}.{prop}"));
                }
            }
        }
        assert!(unread.is_empty(), "schema advertises params the handler never reads: {unread:?}");
    }
}
