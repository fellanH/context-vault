mod corpus;
pub mod fts_token;
pub(crate) mod sqlite_lane;

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;

pub use corpus::{generate_corpus, CorpusSummary};
pub use fts_token::FtsPickResult;
pub use sqlite_lane::SqliteLane;

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiskMode {
    #[default]
    A,
    B,
    C,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkloadMix {
    #[default]
    ReadHeavy,
    WriteHeavy,
    Balanced,
}

#[derive(Debug)]
pub struct BenchArgs {
    pub disk_mode: DiskMode,
    pub workload: WorkloadMix,
    pub seed: u64,
    pub corpus_root: Option<PathBuf>,
    pub corpus_entries: u32,
    pub schema_fixture_rel: &'static str,
    pub vault_db: Option<PathBuf>,
    pub archive_json: Option<PathBuf>,
}

#[derive(Serialize)]
pub struct BenchOutput {
    #[serde(rename = "bench-meta")]
    pub bench_meta: BenchMeta,
    pub timings: Timings,
    pub corpus: CorpusSummary,
    pub disk_modes: DiskModesDoc,
    pub workload_mixes: WorkloadMixesDoc,
}

#[derive(Serialize)]
pub struct BenchMeta {
    pub cli_version: String,
    pub schema_fixture: String,
    pub disk_mode: DiskMode,
    pub workload_mix: WorkloadMix,
    pub seed: u64,
    pub measurement_note: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault_db: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault_fts_limits: Option<&'static str>,
}

#[derive(Serialize)]
pub struct Timings {
    pub corpus_generation_ms: f64,
    pub sqlite: SqliteLane,
    pub total_ms: f64,
}

#[derive(Serialize)]
pub struct DiskModesDoc {
    pub a: &'static str,
    pub b: &'static str,
    pub c: &'static str,
}

#[derive(Serialize)]
pub struct WorkloadMixesDoc {
    pub read_heavy: &'static str,
    pub write_heavy: &'static str,
    pub balanced: &'static str,
}

pub fn run_bench(args: BenchArgs) -> anyhow::Result<BenchOutput> {
    let t_total = Instant::now();
    let t0 = Instant::now();
    let corpus = generate_corpus(
        args.corpus_root.clone(),
        args.seed,
        args.corpus_entries,
        args.disk_mode,
    )?;
    let corpus_generation_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let sqlite = if let Some(ref db_path) = args.vault_db {
        let conn = sqlite_lane::open_user_vault(db_path)?;
        sqlite_lane::run_user_vault_read_probes(&conn, args.workload, args.disk_mode)?
    } else {
        let (conn, path) =
            sqlite_lane::materialize_and_seed(args.disk_mode, args.seed, args.corpus_entries)?;
        sqlite_lane::run_fixture_probes(&conn, args.workload, args.disk_mode, path.as_deref())?
    };

    let vault_db = args
        .vault_db
        .as_ref()
        .map(|p| p.display().to_string());
    let vault_fts_limits = if vault_db.is_some() {
        Some(
            "User vault FTS probe: see timings.sqlite.fts_user_probe for token strategy and hard caps; not comparable to fixture MATCH 'hybrid'.",
        )
    } else {
        None
    };

    let bench_meta = BenchMeta {
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
        schema_fixture: args.schema_fixture_rel.to_string(),
        disk_mode: args.disk_mode,
        workload_mix: args.workload,
        seed: args.seed,
        measurement_note: "SQLite timings are wall-clock for stated SQL; corpus timing is disk write lane. Mode A = in-memory DB + no per-file fsync; B = file-backed DB (WAL+FULL during seed) + fsync per markdown file; C = DELETE journal + FULL + fsync per file.",
        vault_db,
        vault_fts_limits,
    };

    let out = BenchOutput {
        bench_meta,
        timings: Timings {
            corpus_generation_ms,
            sqlite,
            total_ms: t_total.elapsed().as_secs_f64() * 1000.0,
        },
        corpus,
        disk_modes: DiskModesDoc {
            a: "sqlite:memory + corpus without fsync — compare markdown grep with rg (see sqlite.mode_a_rg_equivalent)",
            b: "sqlite:temp file WAL+FULL for seed, then measured probes — primary tuning lane",
            c: "sqlite:temp file DELETE+FULL + fsync each corpus file — supplementary stress",
        },
        workload_mixes: WorkloadMixesDoc {
            read_heavy: "fixture: single-row insert probe + read queries weighted in docs",
            write_heavy: "fixture: 200-row batched insert transaction + reads",
            balanced: "fixture: 50-row batched insert + reads",
        },
    };

    if let Some(ref archive_path) = args.archive_json {
        if let Some(parent) = archive_path.parent() {
            fs::create_dir_all(parent).map_err(|e| anyhow::anyhow!("archive dir: {e}"))?;
        }
        let bytes = serde_json::to_vec_pretty(&out)?;
        fs::write(archive_path, bytes)
            .map_err(|e| anyhow::anyhow!("write archive {}: {e}", archive_path.display()))?;
    }

    Ok(out)
}
