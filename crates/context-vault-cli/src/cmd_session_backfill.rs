//! Card 23 backfill: the SAME writer as live `session-end`, run in batch mode
//! over a directory of transcript `.jsonl` files instead of one hook payload on
//! stdin. Idempotent -- re-running over the same directory skips sessions that
//! already have a log.

use std::path::Path;

use anyhow::{Context, Result};
use context_vault_core::backends::SqliteBackend;
use cv_ingest::ingest::{build_signals, extract_insights, normalize_turns, parse_turns, HookInput, SessionSignals};

use crate::cmd_session_end::{save_new_insights, save_session_entry};
use crate::engine_cmd;
use crate::session_repo_log;

const INSIGHT_PATTERNS: &[&str] = &["★ Insight"];
const INSIGHT_MIN_CHARS: usize = 50;
const INSIGHT_MAX_PER_SESSION: usize = 5;

pub async fn run_backfill(transcripts_dir: &str, cwd_root: &str) -> Result<()> {
    let dir = Path::new(transcripts_dir);
    anyhow::ensure!(dir.is_dir(), "not a directory: {transcripts_dir}");
    let engine = engine_cmd::build_engine().await.context("building vault engine")?;

    let mut counts = (0usize, 0usize, 0usize); // written, skipped, failed
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {transcripts_dir}"))? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        record_outcome(&mut counts, backfill_one(&engine, &path, cwd_root).await, &path);
    }
    println!("backfill {cwd_root}: {} written, {} skipped, {} failed", counts.0, counts.1, counts.2);
    Ok(())
}

fn record_outcome(counts: &mut (usize, usize, usize), result: Result<bool>, path: &Path) {
    match result {
        Ok(true) => counts.0 += 1,
        Ok(false) => counts.1 += 1,
        Err(e) => {
            counts.2 += 1;
            eprintln!("[context-vault] backfill: {}: {e:#}", path.display());
        }
    }
}

/// `Ok(true)` if a log was written, `Ok(false)` if the session had no user
/// turns or was already backfilled.
async fn backfill_one(engine: &SqliteBackend, path: &Path, cwd_root: &str) -> Result<bool> {
    let session_id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown-session").to_string();
    let raw = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let turns = normalize_turns(parse_turns(&raw));
    let Some(signals) = build_signals(&turns, cwd_root) else {
        return Ok(false);
    };

    let seat = synthetic_seat(&session_id);
    let (started, ended) = session_repo_log::session_timestamps(&turns);
    if session_repo_log::log_already_exists(cwd_root, &seat, &session_id, started.as_deref()) {
        return Ok(false);
    }

    let candidates = extract_insights(&turns, INSIGHT_PATTERNS, INSIGHT_MIN_CHARS, INSIGHT_MAX_PER_SESSION);
    let log_path = write_log(cwd_root, &session_id, &seat, started.as_deref(), ended.as_deref(), path, &signals, &candidates)?;

    let input = HookInput { session_id: Some(session_id.clone()), transcript_path: path.to_string_lossy().to_string(), cwd: cwd_root.to_string() };
    save_session_entry(engine, &input, &signals, Some(&log_path)).await?;
    let _ = save_new_insights(engine, &candidates, &signals.project).await;
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn write_log(
    cwd_root: &str,
    session_id: &str,
    seat: &str,
    started: Option<&str>,
    ended: Option<&str>,
    transcript_path: &Path,
    signals: &SessionSignals,
    candidates: &[cv_ingest::ingest::InsightCandidate],
) -> Result<std::path::PathBuf> {
    let metrics = session_repo_log::fetch_fleet_metrics(&transcript_path.to_string_lossy());
    let args = session_repo_log::SessionLogArgs {
        cwd: cwd_root,
        session_id,
        seat,
        started_at: started,
        ended_at: ended,
        metrics: &metrics,
        signals,
        insights: candidates,
    };
    session_repo_log::write_session_log(&args)
}

/// Backfill has no live pane to ask (unlike the live path's `resolve_seat`,
/// which would otherwise misattribute every backfilled session to whichever
/// pane happens to be running the backfill). Falls back to the session id.
fn synthetic_seat(session_id: &str) -> String {
    session_id.chars().filter(|c| *c != '-').take(8).collect::<String>().to_uppercase()
}
