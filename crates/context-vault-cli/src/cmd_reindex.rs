//! `context-vault reindex` on v5 (card 34): a full run states its plan first and refuses
//! when nobody is watching and the estimate is over budget. v4's reindex is unchanged.
use anyhow::{bail, Result};
use context_vault_core::engine::{ReindexOptions, VaultEngine};
use cv_index::engine::V5Backend;
use std::io::IsTerminal;

pub struct Flags {
    pub full: bool,
    pub plan: bool,
    pub json: bool,
    pub max_seconds: u64,
}

pub async fn run(flags: Flags) -> Result<()> {
    let Some(engine) = V5Backend::from_registry()? else {
        if flags.plan {
            bail!("--plan needs the v5 engine (run `context-vault migrate` first)");
        }
        return crate::engine_cmd::run_reindex(flags.full).await;
    };
    if flags.full || flags.plan {
        check_plan(&engine, &flags)?;
        if flags.plan {
            return Ok(());
        }
    }
    let summary = engine.reindex(ReindexOptions { full: flags.full }).await?;
    println!("indexed {} records, removed {}, visited {} sources, in {:.0}ms", summary.entries_reindexed, summary.rows_removed, summary.files_seen, summary.duration_ms);
    Ok(())
}

/// Prints the plan; refuses an unattended run whose estimate exceeds `--max-seconds`.
fn check_plan(engine: &V5Backend, flags: &Flags) -> Result<()> {
    let plan = engine.full_index_plan()?;
    let est = plan["est_seconds"].as_f64().unwrap_or(0.0);
    if flags.json || flags.plan {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        for v in plan["vaults"].as_array().into_iter().flatten() {
            eprintln!("plan: {}: {} files, {:.1} MB, about {} s ({})", v["name"].as_str().unwrap_or(""), v["files"], v["bytes"].as_f64().unwrap_or(0.0) / 1e6, v["est_seconds"], v["basis"].as_str().unwrap_or(""));
        }
    }
    let unattended = !std::io::stdin().is_terminal();
    if !flags.plan && unattended && est > flags.max_seconds as f64 {
        bail!("estimated {est:.0} s is over --max-seconds {}; to proceed: context-vault reindex --full --max-seconds {}", flags.max_seconds, (est * 1.3).ceil());
    }
    Ok(())
}
