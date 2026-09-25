//! `context-vault session-end` — the Claude Code SessionEnd hook target. Reads the
//! hook's stdin payload (`{session_id, transcript_path, cwd}`), parses the transcript,
//! and captures a session summary + any auto-extracted insights. Ports the legacy TS
//! hook's behavior (see specs/session-end-port.md); parsing itself lives in
//! `cv_ingest::ingest::{session_transcript, session_insights}` so it's unit-testable
//! without a live vault.
//!
//! Never blocks session end -- a broken hook must not fail the user's session-end
//! flow, so any error here still exits 0 and prints nothing on stdout. It is not
//! silent to stderr, though: each failure is logged there with which step broke,
//! since a hook nobody can diagnose stays broken (this hook went dark for ~6 weeks
//! before anyone noticed, precisely because it had no failure signal at all).

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use context_vault_core::backends::SqliteBackend;
use context_vault_core::engine::{ListOptions, SaveInput, VaultEngine};
use cv_ingest::ingest::{
    build_signals, extract_insights, format_session_body, normalize_turns, parse_hook_input,
    parse_turns, HookInput, InsightCandidate, SessionSignals,
};
use serde_json::Value;

use crate::engine_cmd;
use crate::session_repo_log;

const INSIGHT_PATTERNS: &[&str] = &["★ Insight"];
const INSIGHT_MIN_CHARS: usize = 50;
const INSIGHT_MAX_PER_SESSION: usize = 5;
const INSIGHT_TIER: &str = "working";

pub async fn run_session_end() {
    match try_capture().await {
        Ok(Some((session_id, insights_saved))) => print_capture_result(&session_id, insights_saved),
        Ok(None) => {}
        Err(e) => eprintln!("[context-vault] session-end: {e:#}"),
    }
}

fn print_capture_result(session_id: &str, insights_saved: usize) {
    println!("context-vault session captured — id: {session_id}");
    if insights_saved > 0 {
        let plural = if insights_saved == 1 { "" } else { "s" };
        println!("context-vault auto-insights — {insights_saved} insight{plural} saved");
    }
}

async fn try_capture() -> Result<Option<(String, usize)>> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).context("reading stdin hook payload")?;
    let Some(input) = parse_hook_input(raw.trim()) else {
        anyhow::bail!("parsing hook payload (need session_id/transcript_path/cwd): {raw:.200}");
    };
    let transcript_raw = std::fs::read_to_string(&input.transcript_path)
        .with_context(|| format!("reading transcript at {}", input.transcript_path))?;
    let turns = normalize_turns(parse_turns(&transcript_raw));
    let Some(signals) = build_signals(&turns, &input.cwd) else {
        // No user turns in the transcript -- nothing worth capturing, not a failure.
        return Ok(None);
    };
    let candidates = extract_insights(&turns, INSIGHT_PATTERNS, INSIGHT_MIN_CHARS, INSIGHT_MAX_PER_SESSION);
    let log_path = repo_log_or_log_error(&input, &turns, &signals, &candidates);

    let engine = engine_cmd::build_engine().await.context("building vault engine")?;
    let session_id = save_session_entry(&engine, &input, &signals, log_path.as_deref())
        .await
        .context("saving session entry")?;
    let insights_saved = save_new_insights(&engine, &candidates, &signals.project).await.unwrap_or(0);
    Ok(Some((session_id, insights_saved)))
}

/// The repo session log (card 23) is the authoritative artifact, but a write
/// failure there (e.g. an unwritable project dir) must not cost the whole
/// capture -- fall back to `None` and let `save_session_entry` keep the old
/// full-body vault entry instead of a pointer.
fn repo_log_or_log_error(
    input: &HookInput,
    turns: &[Value],
    signals: &SessionSignals,
    candidates: &[InsightCandidate],
) -> Option<PathBuf> {
    let seat = session_repo_log::resolve_seat(input.session_id.as_deref().unwrap_or("unknown-session"));
    let metrics = session_repo_log::fetch_fleet_metrics(&input.transcript_path);
    let (started, ended) = session_repo_log::session_timestamps(turns);
    let args = session_repo_log::SessionLogArgs {
        cwd: &input.cwd,
        session_id: input.session_id.as_deref().unwrap_or("unknown-session"),
        seat: &seat,
        started_at: started.as_deref(),
        ended_at: ended.as_deref(),
        metrics: &metrics,
        signals,
        insights: candidates,
    };
    match session_repo_log::write_session_log(&args) {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("[context-vault] session-end: writing repo session log: {e:#}");
            None
        }
    }
}

/// The repo log (card 23) is the authoritative artifact once it exists -- the
/// vault row becomes a pointer to it, not a second copy of its content. Falls
/// back to the old full-body entry only when the log write itself failed.
pub(crate) async fn save_session_entry(
    engine: &SqliteBackend,
    input: &HookInput,
    signals: &SessionSignals,
    log_path: Option<&Path>,
) -> anyhow::Result<String> {
    let body = match log_path {
        Some(p) => pointer_body(p, signals),
        None => format_session_body(signals),
    };
    let title = format!(
        "Session — {} {}",
        signals.project,
        chrono::Local::now().format("%b %-d, %Y %H:%M")
    );
    let mut meta = serde_json::json!({
        "session_id": input.session_id,
        "cwd": input.cwd,
        "message_count": signals.message_count,
    });
    if let Some(p) = log_path {
        meta["session_log_path"] = serde_json::json!(p.to_string_lossy());
    }

    let out = engine
        .save(SaveInput {
            kind: "session".to_string(),
            title: Some(title),
            body,
            tags: Some(vec![
                "session-end".to_string(),
                "session-summary".to_string(),
                signals.project.clone(),
            ]),
            source: Some("claude-code".to_string()),
            meta: Some(meta),
            ..Default::default()
        })
        .await?;
    Ok(out.id)
}

fn pointer_body(log_path: &Path, signals: &SessionSignals) -> String {
    format!(
        "Session log: {}\n\n{} — {} exchanges. {}",
        log_path.display(),
        signals.project,
        signals.message_count,
        signals.first_prompt
    )
}

pub(crate) async fn save_new_insights(
    engine: &SqliteBackend,
    candidates: &[InsightCandidate],
    project: &str,
) -> anyhow::Result<usize> {
    if candidates.is_empty() {
        return Ok(0);
    }

    let mut seen = existing_auto_insight_titles(engine).await?;
    let mut saved = 0;
    for candidate in candidates {
        let key = candidate.title.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        save_one_insight(engine, &candidate.title, &candidate.body, project).await?;
        seen.insert(key);
        saved += 1;
    }
    Ok(saved)
}

async fn save_one_insight(
    engine: &SqliteBackend,
    title: &str,
    body: &str,
    project: &str,
) -> anyhow::Result<()> {
    engine
        .save(SaveInput {
            kind: "insight".to_string(),
            title: Some(title.to_string()),
            body: body.to_string(),
            tags: Some(vec![
                "auto-insight".to_string(),
                "session-insight".to_string(),
                format!("bucket:{project}"),
            ]),
            source: Some(format!("claude-code session {}", chrono::Local::now().format("%Y-%m-%d"))),
            tier: Some(INSIGHT_TIER.to_string()),
            meta: Some(serde_json::json!({ "auto_extracted": true })),
            ..Default::default()
        })
        .await?;
    Ok(())
}

/// Last 100 insight-kind entries tagged `auto-insight`, by lowercased title --
/// matches the legacy hook's lightweight title-based dedup.
async fn existing_auto_insight_titles(engine: &SqliteBackend) -> anyhow::Result<HashSet<String>> {
    let entries = engine
        .list(ListOptions {
            kind: Some("insight".to_string()),
            limit: 100,
            ..Default::default()
        })
        .await?;
    Ok(entries
        .into_iter()
        .filter(|e| e.tags.iter().any(|t| t == "auto-insight"))
        .filter_map(|e| e.title)
        .map(|t| t.to_lowercase())
        .collect())
}
