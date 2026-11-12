//! `context-vault recall-hint` — the Claude Code UserPromptSubmit hook target. Reads
//! the hook's stdin payload (`{prompt, cwd, ...}`), runs the prompt through the same
//! recall logic the MCP `recall` tool and the CLI `recall` subcommand use
//! (`context_vault_core::mcp::recall`, which wraps `handlers_read::handle_recall`),
//! and — if anything relevant turns up — emits it as
//! `hookSpecificOutput.additionalContext` JSON on stdout for Claude Code to fold into
//! context.
//!
//! Never fails loudly: prints nothing on stdout unless there's a real hint to
//! surface, and always exits 0. Mirrors `cmd_session_end`'s stderr-log-but-never-block
//! shape so a broken recall path is diagnosable without ever blocking a prompt.

use std::io::Read;
use std::sync::Arc;

use anyhow::{Context, Result};
use context_vault_core::engine::VaultEngine;
use serde_json::Value;

use crate::engine_cmd;

const MAX_HINTS: usize = 3;
const MIN_PROMPT_CHARS: usize = 10;

pub async fn run_recall_hint() {
    match try_recall().await {
        Ok(Some((event, hints))) => emit_additional_context(&event, &hints),
        Ok(None) => {}
        Err(e) => eprintln!("[context-vault] recall-hint: {e:#}"),
    }
}

async fn try_recall() -> Result<Option<(String, String)>> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).context("reading stdin hook payload")?;
    let v: Value = serde_json::from_str(raw.trim()).context("parsing hook payload JSON")?;
    if v.get("hook_event_name").and_then(|e| e.as_str()) == Some("SessionStart") {
        return crate::cmd_recall_situation::session_start_context(&v).await;
    }
    let prompt = v.get("prompt").and_then(|p| p.as_str()).unwrap_or("").trim().to_string();

    if !worth_recalling(&prompt) {
        return Ok(None);
    }

    // Scope hints to the project the session is running in — without this, any
    // session recalls every other project's vault content (observed as what looked
    // like a prompt-injection: an unrelated session being handed another project's
    // "adopt this charter" event). Same project-from-cwd convention as
    // `cv_ingest::build_signals`.
    let scope = v
        .get("cwd")
        .and_then(|c| c.as_str())
        .and_then(project_from_cwd);

    let engine: Arc<dyn VaultEngine> =
        engine_cmd::build_engine().await.context("building vault engine")?;
    let hints = context_vault_core::mcp::recall(engine, &prompt, scope.as_deref(), MAX_HINTS)
        .await
        .map_err(|e| anyhow::anyhow!(e))
        .context("running recall")?;
    Ok(useful_hints(&hints).map(|h| ("UserPromptSubmit".to_string(), h)))
}

/// Last path component of the session cwd, the vault's project/bucket name
/// convention (matches `cv_ingest::build_signals`).
pub fn project_from_cwd(cwd: &str) -> Option<String> {
    let name = cwd.trim_end_matches('/').rsplit('/').next().unwrap_or("").trim();
    if name.is_empty() { None } else { Some(name.to_string()) }
}

/// Cheap pre-filters (mirrors the legacy JS hook): skip slash commands and prompts
/// too short to carry a meaningful recall signal, so a no-op prompt never pays for
/// a vault search.
fn worth_recalling(prompt: &str) -> bool {
    !prompt.is_empty() && !prompt.starts_with('/') && prompt.len() >= MIN_PROMPT_CHARS
}

/// `handle_recall` returns a literal "No hints found for: ..." rather than an empty
/// string when nothing matches (useful for the MCP tool/CLI, where the caller asked
/// and wants to know that). Here there's no caller asking -- injecting that into
/// every miss would be exactly the noise "graceful empty" is meant to avoid.
fn useful_hints(hints: &str) -> Option<String> {
    let hints = hints.trim();
    if hints.is_empty() || hints.starts_with("No hints found for:") {
        None
    } else {
        Some(hints.to_string())
    }
}

fn emit_additional_context(event: &str, hints: &str) {
    let payload = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": event,
            "additionalContext": hints,
        }
    });
    if let Ok(s) = serde_json::to_string(&payload) {
        println!("{s}");
    }
}
