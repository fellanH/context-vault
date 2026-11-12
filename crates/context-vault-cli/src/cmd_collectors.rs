//! The collectors part of `doctor` (cards 04/05): one line per registered collector,
//! then the sources writing into the vault that are not registered (contract violations).

use anyhow::Result;
use chrono::{Duration, Utc};
use cv_vault::collectors::{launchd_label, load, parse_time, verdict, Collector, LastRun};
use rusqlite::{Connection, OpenFlags, OptionalExtension};

const VIOLATION_WINDOW_DAYS: i64 = 30;
/// Below this many rows a source is a one-off provenance note ("call 2026-10-12",
/// a URL), not a writer; those are counted, not listed.
const WRITER_MIN_ROWS: i64 = 10;

pub fn print_collectors(vault_dir: &str, db_path: &str) -> Result<()> {
    let Some(registry) = load(vault_dir)? else {
        println!("collectors: no collectors.toml in {vault_dir}");
        return Ok(());
    };
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    println!("collectors ({}):", registry.collectors.len());
    for (name, c) in &registry.collectors {
        println!("  {}", collector_line(&conn, name, c)?);
    }
    let sources: Vec<&str> = registry.collectors.values().map(|c| c.writes.source.as_str()).collect();
    print_unregistered(&conn, &sources)
}

fn collector_line(conn: &Connection, name: &str, c: &Collector) -> Result<String> {
    let newest: Option<String> = conn
        .query_row("SELECT max(created_at) FROM vault WHERE source = ?1", [&c.writes.source], |r| r.get(0))
        .optional()?
        .flatten();
    let last_run = read_last_run(&c.state);
    let loaded = launchd_label(&c.schedule).and_then(job_loaded);
    let v = verdict(&c.freshness, loaded, newest.as_deref().and_then(parse_time), last_run.as_ref(), Utc::now());
    let run = match &last_run {
        Some(r) if r.ok => format!("last run {} ok", r.ts),
        Some(r) => format!("last run {} FAILED: {}", r.ts, r.error.as_deref().unwrap_or("?")),
        None => "no last-run.json".to_string(),
    };
    let job = match loaded {
        Some(false) => format!(", job {} NOT LOADED (launchctl load it)", c.schedule),
        _ => String::new(),
    };
    Ok(format!(
        "{:<10} {name} (source={}, freshness {}): newest row {}, {run}{job}",
        v.as_str(),
        c.writes.source,
        c.freshness,
        newest.as_deref().unwrap_or("never"),
    ))
}

/// `None` where launchctl can't answer (not macOS, not installed): no claim either way.
fn job_loaded(label: &str) -> Option<bool> {
    let out = std::process::Command::new("launchctl").args(["list", label]).output().ok()?;
    Some(out.status.success())
}

fn read_last_run(state_dir: &str) -> Option<LastRun> {
    let dir = match state_dir.strip_prefix("~/") {
        Some(rest) => dirs::home_dir()?.join(rest),
        None => state_dir.into(),
    };
    serde_json::from_str(&std::fs::read_to_string(dir.join("last-run.json")).ok()?).ok()
}

/// Sources that wrote in the last 30 days without a registry entry: direct drops or
/// drifted writers. The contract says every write names its registered source.
fn print_unregistered(conn: &Connection, registered: &[&str]) -> Result<()> {
    let since = (Utc::now() - Duration::days(VIOLATION_WINDOW_DAYS)).format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(
        "SELECT coalesce(source, '(no source)'), count(*), max(created_at) FROM vault \
         WHERE created_at >= ?1 GROUP BY source ORDER BY 2 DESC",
    )?;
    let rows = stmt.query_map([&since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))?;
    let (unregistered, notes): (Vec<_>, Vec<_>) = rows
        .filter_map(|r| r.ok())
        .filter(|(s, _, _)| !registered.contains(&s.as_str()))
        .partition(|(_, n, _)| *n >= WRITER_MIN_ROWS);
    if !notes.is_empty() {
        println!("one-off provenance sources (< {WRITER_MIN_ROWS} rows each, not collectors): {}", notes.len());
    }
    if unregistered.is_empty() {
        println!("unregistered writers (last {VIOLATION_WINDOW_DAYS} days): none");
        return Ok(());
    }
    println!("unregistered writers (last {VIOLATION_WINDOW_DAYS} days, contract violation: register in collectors.toml or fix the writer's --source):");
    for (source, n, newest) in unregistered {
        println!("  {source}: {n} rows, newest {newest}");
    }
    Ok(())
}
