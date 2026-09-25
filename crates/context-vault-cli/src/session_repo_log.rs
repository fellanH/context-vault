//! Card 23 (`omni-os/pm/doing/23-project-session-logs.md`): writes the durable
//! per-project session log into `<cwd>/project-management/session-logs/`. Shared
//! by the live `session-end` path and `--backfill`, so both are the same writer
//! in different modes -- never two writers for the same artifact.
//!
//! `fleet time` (a pure read-model over transcripts, not this process) supplies
//! the frontmatter metrics; this module never computes timing itself.
//!
//! PUBLIC-REPO GUARD (canon: `~/.claude/rules/behavior/public-repo-boundary.md`):
//! Before writing, resolve the write-target repo's remote visibility and REFUSE
//! (fail-closed) if public or unknown. No remote = private by definition = allow.
//! Refusals are LOUD: one receipt line to stderr + vault row records the refusal.
//! Visibility check delegated to `repo_visibility` module.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use cv_ingest::ingest::{format_session_log, FleetMetrics, InsightCandidate, SessionLogInput, SessionSignals};
use serde_json::Value;

use crate::repo_visibility;

/// iTerm2 sets `ITERM_SESSION_ID=w<N>t<N>p<N>:<GUID>` in every pane's shell env,
/// inherited by hook subprocesses. The first 8 hex chars of the GUID are exactly
/// the `sid` fleet/it2 already use to name seats (e.g. this session is 663425DA)
/// -- reuse that identity rather than inventing a second one. Falls back to the
/// session uuid's own first 8 chars when not running inside iTerm2 (or for
/// backfill, where there's no live pane to ask).
pub fn resolve_seat(session_id: &str) -> String {
    std::env::var("ITERM_SESSION_ID")
        .ok()
        .and_then(|v| v.split(':').nth(1).map(str::to_string))
        .map(|guid| guid.chars().filter(char::is_ascii_hexdigit).collect::<String>())
        .filter(|hex| hex.len() >= 8)
        .map(|hex| hex[..8].to_uppercase())
        .unwrap_or_else(|| session_id.chars().filter(char::is_ascii_hexdigit).take(8).collect::<String>().to_uppercase())
}

/// Runs `fleet time --transcript <path> --window all --json --slow`. `fleet` is
/// a first-party CLI on this machine's PATH; never fails loudly -- a missing
/// binary, a parse error, or "no transcript found" all degrade to an empty
/// metrics block rather than blocking the write.
pub fn fetch_fleet_metrics(transcript_path: &str) -> FleetMetrics {
    match run_fleet_time(transcript_path) {
        Ok(v) => FleetMetrics::from_json(&v),
        Err(e) => {
            eprintln!("[context-vault] session-log: fleet time unavailable for {transcript_path}: {e:#}");
            FleetMetrics::default()
        }
    }
}

fn run_fleet_time(transcript_path: &str) -> Result<Value> {
    let output = Command::new("fleet")
        .args(["time", "--transcript", transcript_path, "--window", "all", "--json", "--slow"])
        .output()
        .context("spawning `fleet time`")?;
    if !output.status.success() {
        anyhow::bail!("fleet time exited {}: {}", output.status, String::from_utf8_lossy(&output.stderr));
    }
    serde_json::from_slice(&output.stdout).context("parsing fleet time JSON")
}

/// First/last turn `timestamp` fields, if the transcript carried them.
pub fn session_timestamps(turns: &[Value]) -> (Option<String>, Option<String>) {
    let ts = |t: &Value| t.get("timestamp").and_then(|v| v.as_str()).map(str::to_string);
    (turns.first().and_then(ts), turns.last().and_then(ts))
}

pub struct SessionLogArgs<'a> {
    pub cwd: &'a str,
    pub session_id: &'a str,
    pub seat: &'a str,
    pub started_at: Option<&'a str>,
    pub ended_at: Option<&'a str>,
    pub metrics: &'a FleetMetrics,
    pub signals: &'a SessionSignals,
    pub insights: &'a [InsightCandidate],
}

/// `<cwd>/project-management/session-logs/<date>-<seat>-<session8>.md`. `cwd`
/// is used verbatim as the project root (not walked up to a git root): a
/// monorepo sub-project (e.g. acme/projects/billing) is its own logical
/// project here even though it shares a `.git` with its parent.
pub fn log_path_for(cwd: &str, seat: &str, session_id: &str, started_at: Option<&str>) -> PathBuf {
    let date = started_at.map(|s| &s[..10.min(s.len())]).unwrap_or("unknown-date");
    let session8: String = session_id.chars().filter(|c| *c != '-').take(8).collect();
    Path::new(cwd).join("project-management").join("session-logs").join(format!("{date}-{seat}-{session8}.md"))
}

/// True if this session already has a log (backfill idempotency: re-running
/// over the same transcript directory is a no-op for already-written sessions).
pub fn log_already_exists(cwd: &str, seat: &str, session_id: &str, started_at: Option<&str>) -> bool {
    log_path_for(cwd, seat, session_id, started_at).is_file()
}


pub fn write_session_log(args: &SessionLogArgs) -> Result<PathBuf> {
    // PUBLIC-REPO GUARD: refuse if target repo is public or unknown
    match repo_visibility::repo_visibility_is_public(args.cwd) {
        Ok(false) => {
            // Private or no remotes — proceed
        }
        Ok(true) => {
            // Public repo — refuse loudly
            let repo_path = std::fs::canonicalize(args.cwd)
                .unwrap_or_else(|_| PathBuf::from(args.cwd))
                .to_string_lossy()
                .to_string();
            let receipt = format!("[context-vault] public-repo-guard: REFUSED — public repo: {repo_path}");
            eprintln!("{receipt}");
            anyhow::bail!("{receipt}");
        }
        Err(e) => {
            // Unknown visibility — fail closed, refuse loudly
            let receipt = format!("[context-vault] public-repo-guard: REFUSED — unable to verify repo visibility: {e}");
            eprintln!("{receipt}");
            anyhow::bail!("{receipt}");
        }
    }

    let path = log_path_for(args.cwd, args.seat, args.session_id, args.started_at);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let input = SessionLogInput {
        session_id: args.session_id,
        seat: args.seat,
        cwd: args.cwd,
        started_at: args.started_at,
        ended_at: args.ended_at,
        turns: args.signals.message_count,
        metrics: args.metrics,
        signals: args.signals,
        insights: args.insights,
    };
    std::fs::write(&path, format_session_log(&input)).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}
