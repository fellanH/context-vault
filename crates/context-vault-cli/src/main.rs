mod engine_cmd;
mod cmd_read;
mod cmd_write;
mod cmd_lifecycle;
mod cmd_ingest;
mod cmd_setup;
mod cmd_schema;
mod cmd_move;
mod cmd_session_end;
mod cmd_session_backfill;
mod cmd_recall_hint;
mod session_repo_log;
mod repo_visibility;
mod client_paths;
mod setup_register;
mod cli;

#[cfg(feature = "dev-tools")]
mod ipc_invoke;

use cli::{Cli, Commands, SchemaCommands, VaultCommands};
use clap::Parser;

#[cfg(feature = "dev-tools")]
use std::io::{self, Write};
#[cfg(feature = "dev-tools")]
use std::path::PathBuf;
#[cfg(feature = "dev-tools")]
use anyhow::anyhow;
#[cfg(feature = "dev-tools")]
use context_vault_core::bench_harness;

use anyhow::Result;

#[cfg(feature = "dev-tools")]
const SCHEMA_FIXTURE_REL: &str = "fixtures/bench/reference_schema.sql";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => {
            println!(
                "context-vault Rust slice {} — use `context-vault --help`",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        Some(cmd) => dispatch(cmd).await,
    }
}

async fn dispatch(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Setup { client, print, dry_run } => {
            cmd_setup::run_setup(cmd_setup::SetupOpts { client, print, dry_run }).await
        }
        #[cfg(feature = "dev-tools")]
        Commands::Bench {
            format,
            disk_mode,
            workload,
            seed,
            corpus_root,
            corpus_entries,
            vault_db,
            archive_json,
        } => run_bench(
            &format, disk_mode, workload, seed, corpus_root, corpus_entries, vault_db,
            archive_json,
        ),
        Commands::Mcp => context_vault_core::mcp::run().await,
        Commands::Reindex { full } => engine_cmd::run_reindex(full).await,
        Commands::Status => engine_cmd::run_status().await,
        Commands::Doctor => engine_cmd::run_doctor().await,
        Commands::Vault { command } => match command {
            VaultCommands::List => engine_cmd::run_vault_list(),
            VaultCommands::Use { name } => engine_cmd::run_vault_use(&name),
        },
        Commands::Schema { command } => match command {
            SchemaCommands::Init => cmd_schema::run_schema_init(),
            SchemaCommands::Show => cmd_schema::run_schema_show(),
            SchemaCommands::Apply { dry_run } => cmd_schema::run_schema_apply(dry_run).await,
        },
        Commands::Move { source, dest, dry_run } => cmd_move::run_move(source, dest, dry_run).await,
        Commands::Save { kind, title, body, tags, tier, source, meta, identity_key } => {
            cmd_read::run_save(kind, title, body, tags, tier, source, meta, identity_key).await
        }
        Commands::Get { id } => cmd_read::run_get(&id).await,
        Commands::Search { query, limit, format } => cmd_read::run_search(&query, limit, &format).await,
        Commands::List { kind, tier, limit } => cmd_read::run_list(kind, tier, limit).await,
        Commands::Recall { signal, limit, bucket, global } => cmd_read::run_recall(&signal, limit, bucket, global).await,
        Commands::Delete { id } => cmd_write::run_delete(&id).await,
        Commands::Stats => engine_cmd::run_stats().await,
        Commands::Ingest { url } => cmd_ingest::run_ingest(&url).await,
        Commands::IngestProject { path } => cmd_ingest::run_ingest_project(&path).await,
        Commands::Archive { days } => cmd_lifecycle::run_archive(days).await,
        Commands::Restore { id } => cmd_lifecycle::run_restore(&id).await,
        Commands::Summarize => cmd_lifecycle::run_summarize().await,
        Commands::Tier => cmd_lifecycle::run_tier().await,
        Commands::Assemble { ids } => cmd_lifecycle::run_assemble(ids).await,
        Commands::PostToolCall => Ok(()),
        Commands::SessionEnd { backfill: None, .. } => {
            cmd_session_end::run_session_end().await;
            Ok(())
        }
        Commands::SessionEnd { backfill: Some(dir), cwd_root } => {
            let cwd_root = cwd_root.ok_or_else(|| anyhow::anyhow!("--backfill requires --cwd-root"))?;
            cmd_session_backfill::run_backfill(&dir, &cwd_root).await
        }
        Commands::RecallHint => {
            cmd_recall_hint::run_recall_hint().await;
            Ok(())
        }
        #[cfg(feature = "dev-tools")]
        Commands::Invoke { stdin_json } => ipc_invoke::run(stdin_json),
    }
}

#[cfg(feature = "dev-tools")]
#[allow(clippy::too_many_arguments)]
fn run_bench(
    format: &str,
    disk_mode: bench_harness::DiskMode,
    workload: bench_harness::WorkloadMix,
    seed: u64,
    corpus_root: Option<PathBuf>,
    corpus_entries: u32,
    vault_db: Option<PathBuf>,
    archive_json: Option<PathBuf>,
) -> Result<()> {
    if format != "json" {
        return Err(anyhow!("only --format json is supported in this slice"));
    }
    let out = bench_harness::run_bench(bench_harness::BenchArgs {
        disk_mode,
        workload,
        seed,
        corpus_root,
        corpus_entries,
        schema_fixture_rel: SCHEMA_FIXTURE_REL,
        vault_db,
        archive_json,
    })?;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    serde_json::to_writer_pretty(&mut stdout, &out)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
