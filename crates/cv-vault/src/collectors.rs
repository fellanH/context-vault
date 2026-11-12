//! The collector registry (`collectors.toml`, next to vault.toml) and the doctor's
//! verdict per collector. Contract: pm/intake-contract.md (ratified 2026-07-08 D5).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Deserialize;

pub const COLLECTORS_FILENAME: &str = "collectors.toml";

#[derive(Debug, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub collectors: BTreeMap<String, Collector>,
}

#[derive(Debug, Deserialize)]
pub struct Collector {
    pub owner: String,
    pub schedule: String,
    pub writes: Writes,
    pub freshness: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct Writes {
    #[serde(default)]
    pub kinds: Vec<String>,
    pub source: String,
}

/// `<state>/last-run.json`, written by the collector after every run.
#[derive(Debug, Deserialize)]
pub struct LastRun {
    pub ts: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Its scheduler job is registered but not loaded: it will never run again.
    NotLoaded,
    Green,
    Stale,
    Dead,
    Erroring,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::NotLoaded => "not-loaded",
            Verdict::Green => "green",
            Verdict::Stale => "stale",
            Verdict::Dead => "dead",
            Verdict::Erroring => "erroring",
        }
    }
}

/// `None` when the vault has no collectors.toml.
pub fn load(vault_dir: &str) -> Result<Option<Registry>> {
    let path = Path::new(vault_dir).join(COLLECTORS_FILENAME);
    match std::fs::read_to_string(&path) {
        Ok(raw) => Ok(Some(toml::from_str(&raw)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// "24h" / "7d" -> a window; "event" (or anything else) -> `None`: an event-driven leg
/// may stay quiet for weeks, so only an error in its last run counts against it.
pub fn window(freshness: &str) -> Option<Duration> {
    let (n, unit) = freshness.split_at(freshness.len().checked_sub(1)?);
    let n: i64 = n.parse().ok()?;
    match unit {
        "h" => Some(Duration::hours(n)),
        "d" => Some(Duration::days(n)),
        _ => None,
    }
}

/// `launchd:<label>` -> the label; other schedules (hooks, manual) have no job to check.
pub fn launchd_label(schedule: &str) -> Option<&str> {
    schedule.strip_prefix("launchd:").filter(|label| !label.is_empty())
}

/// A job that is not loaded wins: a quiet event leg looks green while it never fires.
/// Then an erroring last run; then age of the newest row against the window. A leg
/// that never wrote a row and never reported a run is dead.
pub fn verdict(
    freshness: &str,
    job_loaded: Option<bool>,
    newest_row: Option<DateTime<Utc>>,
    last_run: Option<&LastRun>,
    now: DateTime<Utc>,
) -> Verdict {
    if job_loaded == Some(false) {
        return Verdict::NotLoaded;
    }
    if last_run.is_some_and(|r| !r.ok) {
        return Verdict::Erroring;
    }
    if newest_row.is_none() && last_run.is_none() {
        return Verdict::Dead;
    }
    match (window(freshness), newest_row) {
        (None, _) => Verdict::Green,
        (Some(_), None) => Verdict::Dead,
        (Some(w), Some(t)) if now - t > w => Verdict::Stale,
        _ => Verdict::Green,
    }
}

/// RFC 3339, `YYYY-MM-DD HH:MM:SS` or a bare date, as UTC.
pub fn parse_time(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return Some(t.with_timezone(&Utc));
    }
    let day = NaiveDate::parse_from_str(s.get(..10)?, "%Y-%m-%d").ok()?;
    Some(day.and_hms_opt(0, 0, 0)?.and_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        parse_time(s).unwrap()
    }

    #[test]
    fn windows_and_event_mode() {
        assert_eq!(window("24h"), Some(Duration::hours(24)));
        assert_eq!(window("7d"), Some(Duration::days(7)));
        assert_eq!(window("event"), None);
        assert_eq!(window(""), None);
    }

    #[test]
    fn verdicts() {
        let now = t("2026-10-19T12:00:00Z");
        let ok = LastRun { ts: "2026-10-19T00:00:00Z".into(), ok: true, error: None };
        let bad = LastRun { ts: "2026-10-19T00:00:00Z".into(), ok: false, error: Some("auth".into()) };
        assert_eq!(verdict("24h", None, Some(t("2026-10-19T08:00:00Z")), None, now), Verdict::Green);
        assert_eq!(verdict("24h", None, Some(t("2026-10-17T08:00:00Z")), None, now), Verdict::Stale);
        assert_eq!(verdict("24h", None, None, None, now), Verdict::Dead);
        assert_eq!(verdict("24h", None, Some(t("2026-10-19T08:00:00Z")), Some(&bad), now), Verdict::Erroring);
        assert_eq!(verdict("event", None, Some(t("2026-01-01")), None, now), Verdict::Green, "quiet event leg is fine");
        assert_eq!(verdict("event", None, None, Some(&ok), now), Verdict::Green, "ran, nothing to save");
        assert_eq!(verdict("event", None, None, None, now), Verdict::Dead, "never ran");
        assert_eq!(
            verdict("event", Some(false), Some(t("2026-10-19T08:00:00Z")), Some(&ok), now),
            Verdict::NotLoaded,
            "a recent row does not hide a job that will never fire again"
        );
        assert_eq!(verdict("event", Some(true), None, Some(&ok), now), Verdict::Green);
        assert_eq!(launchd_label("launchd:com.example.job"), Some("com.example.job"));
        assert_eq!(launchd_label("hook:SessionEnd"), None);
        assert_eq!(launchd_label("launchd:"), None);
    }

    #[test]
    fn parses_the_registry_shape() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(COLLECTORS_FILENAME),
            "schema_version = 1\n[collectors.transcribe]\nowner = \"x\"\nschedule = \"manual\"\nwrites = { kinds = [\"note\"], source = \"transcribe\" }\nfreshness = \"event\"\nstate = \"~/.omni/collectors/transcribe/\"\nprobe = \"newest row\"\n",
        )
        .unwrap();
        let reg = load(tmp.path().to_str().unwrap()).unwrap().unwrap();
        assert_eq!(reg.collectors["transcribe"].writes.source, "transcribe");
        assert!(load(tmp.path().join("nope").to_str().unwrap()).unwrap().is_none());
    }
}
