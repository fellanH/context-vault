//! Estimates for full index runs (card 34): planned bytes over the rate the last full run
//! on this machine measured (stored beside the index), else a labelled guess.
use super::{V5Backend, Vault};
use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;

/// Debug and release builds differ ~20x; this is a cautious release-build figure from the
/// 2026-09-25 G1 run, used only until a run on this machine has been measured.
const GUESS_BYTES_PER_SEC: f64 = 100.0 * 1024.0 * 1024.0;

impl V5Backend {
    /// `{vaults: [{name, files, bytes, est_seconds, basis}], est_seconds}` for a full run.
    pub fn full_index_plan(&self) -> Result<Value> {
        let mut rows = Vec::new();
        let mut total = 0.0;
        for vault in &self.vaults {
            let plan = vault.index.plan()?;
            let (rate, basis) = match measured_rate(vault) {
                Some(rate) => (rate, "measured"),
                None => (GUESS_BYTES_PER_SEC, "guess"),
            };
            let est = plan.bytes as f64 / rate;
            total += est;
            rows.push(json!({ "name": vault.name, "files": plan.files, "bytes": plan.bytes, "est_seconds": est.ceil(), "basis": basis }));
        }
        Ok(json!({ "vaults": rows, "est_seconds": total.ceil() }))
    }
}

fn rate_file(vault: &Vault) -> std::path::PathBuf {
    vault.index_dir.join("rate.json")
}

fn measured_rate(vault: &Vault) -> Option<f64> {
    let raw = std::fs::read_to_string(rate_file(vault)).ok()?;
    serde_json::from_str::<Value>(&raw).ok()?["bytes_per_sec"].as_f64().filter(|r| *r > 0.0)
}

/// After a full run: remember how fast this machine read this vault.
pub(super) fn record_rate(vault: &Vault, bytes: u64, took: Duration) {
    if bytes < 1024 * 1024 || took.as_secs_f64() <= 0.0 {
        return; // too small to say anything about the rate
    }
    let rate = bytes as f64 / took.as_secs_f64();
    let _ = std::fs::write(rate_file(vault), json!({ "bytes_per_sec": rate, "measured_at": chrono::Utc::now().to_rfc3339() }).to_string());
}
