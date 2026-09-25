//! Composes the durable per-project session log (card 23,
//! `omni-os/pm/doing/23-project-session-logs.md`): frontmatter carries machine
//! metrics (from `fleet time`, a pure read-model this module never computes
//! itself), body carries the distilled narrative reusing the same signals the
//! vault capture already builds. The vault row becomes a pointer to this file,
//! not a second copy of its content -- one source per meaning per audience.

use serde_json::Value;

use super::session_insights::InsightCandidate;
use super::session_transcript::{format_session_body, SessionSignals};

/// Bumped whenever the underlying `fleet time` computation changes in a way
/// that would make old and new logs uncomparable (e.g. v0 undercounts
/// subagent time) -- required in every log's frontmatter per the card.
pub const METRICS_VERSION: &str = "fleet-time-v0";

#[derive(Debug, Clone, Default)]
pub struct SlowCall {
    pub tool: String,
    pub latency_s: f64,
    pub detail: String,
}

/// Parsed from `fleet time --transcript <path> --window all --json --slow`.
/// All fields optional: a transcript `fleet time` can't find (e.g. deleted
/// mid-backfill) still produces a log, just without these numbers.
#[derive(Debug, Clone, Default)]
pub struct FleetMetrics {
    pub wall_median_s: Option<f64>,
    pub wall_p90_s: Option<f64>,
    pub calls_per_turn: Option<f64>,
    pub parallel_ratio: Option<f64>,
    pub tool_latency_share_pct: Option<f64>,
    pub top_slow: Vec<SlowCall>,
    pub findings: Vec<String>,
}

impl FleetMetrics {
    pub fn from_json(v: &Value) -> Self {
        let f = |k: &str| v.get(k).and_then(|x| x.as_f64());
        let top_slow = v
            .get("top_slow_calls")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|c| SlowCall {
                        tool: c.get("tool").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        latency_s: c.get("latency_s").and_then(|x| x.as_f64()).unwrap_or(0.0),
                        detail: c.get("detail").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let findings = v
            .get("findings")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| f.get("summary").and_then(|s| s.as_str()))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        FleetMetrics {
            wall_median_s: f("wall_median_s"),
            wall_p90_s: f("wall_p90_s"),
            calls_per_turn: f("calls_avg"),
            parallel_ratio: f("parallel_ratio"),
            tool_latency_share_pct: f("tool_latency_share_pct"),
            top_slow,
            findings,
        }
    }
}

pub struct SessionLogInput<'a> {
    pub session_id: &'a str,
    pub seat: &'a str,
    pub cwd: &'a str,
    pub started_at: Option<&'a str>,
    pub ended_at: Option<&'a str>,
    pub turns: usize,
    pub metrics: &'a FleetMetrics,
    pub signals: &'a SessionSignals,
    pub insights: &'a [InsightCandidate],
}

pub fn format_session_log(input: &SessionLogInput) -> String {
    format!("---\n{}---\n\n{}\n", format_frontmatter(input), format_body(input))
}

fn format_frontmatter(input: &SessionLogInput) -> String {
    let mut lines = vec![
        format!("session: {}", input.session_id),
        format!("seat: {}", input.seat),
        format!("cwd: {:?}", input.cwd),
    ];
    push_opt(&mut lines, "started", input.started_at.map(str::to_string));
    push_opt(&mut lines, "ended", input.ended_at.map(str::to_string));
    lines.push(format!("turns: {}", input.turns));
    lines.push(format!("metrics_version: {METRICS_VERSION}"));
    push_opt(&mut lines, "wall_median_s", input.metrics.wall_median_s.map(|v| format!("{v:.2}")));
    push_opt(&mut lines, "wall_p90_s", input.metrics.wall_p90_s.map(|v| format!("{v:.2}")));
    push_opt(&mut lines, "calls_per_turn", input.metrics.calls_per_turn.map(|v| format!("{v:.2}")));
    push_opt(&mut lines, "parallel_ratio", input.metrics.parallel_ratio.map(|v| format!("{v:.2}")));
    push_opt(&mut lines, "tool_latency_share_pct", input.metrics.tool_latency_share_pct.map(|v| format!("{v:.2}")));
    push_top_slow(&mut lines, &input.metrics.top_slow);
    lines.join("\n") + "\n"
}

fn push_opt(lines: &mut Vec<String>, key: &str, val: Option<String>) {
    let Some(v) = val else { return };
    lines.push(format!("{key}: {v}"));
}

fn push_top_slow(lines: &mut Vec<String>, calls: &[SlowCall]) {
    if calls.is_empty() {
        return;
    }
    lines.push("top_slow:".to_string());
    for c in calls {
        lines.push(format!("  - {{ tool: {:?}, latency_s: {:.1}, detail: {:?} }}", c.tool, c.latency_s, c.detail));
    }
}

fn format_body(input: &SessionLogInput) -> String {
    let mut out = vec!["## What happened".to_string(), String::new(), format_session_body(input.signals)];
    push_decisions_section(&mut out, input.insights);
    push_artifacts_section(&mut out, input.signals);
    push_slow_patterns_section(&mut out, &input.metrics.findings);
    out.join("\n")
}

fn push_decisions_section(out: &mut Vec<String>, insights: &[InsightCandidate]) {
    out.push(String::new());
    out.push("## Decisions".to_string());
    if insights.is_empty() {
        out.push("None auto-extracted this session.".to_string());
        return;
    }
    for i in insights {
        out.push(format!("- **{}** — {}", i.title, truncate(&i.body, 200)));
    }
}

fn push_artifacts_section(out: &mut Vec<String>, signals: &SessionSignals) {
    out.push(String::new());
    out.push("## Artifacts".to_string());
    if signals.files_modified.is_empty() && signals.commands_run.is_empty() {
        out.push("None recorded.".to_string());
        return;
    }
    for f in signals.files_modified.iter().take(30) {
        out.push(format!("- {f}"));
    }
}

fn push_slow_patterns_section(out: &mut Vec<String>, findings: &[String]) {
    out.push(String::new());
    out.push("## Slow patterns".to_string());
    if findings.is_empty() {
        out.push("None flagged.".to_string());
        return;
    }
    for f in findings {
        out.push(format!("- {f}"));
    }
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
