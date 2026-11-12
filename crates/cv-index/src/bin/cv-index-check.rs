//! Maintainer validation surface, deliberately separate from the released v4 CLI.
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use cv_core::VaultType;
use cv_index::{migrate::MigrateOptions, SearchOptions, UpdateOptions, VaultIndex};
use std::path::{Path, PathBuf};

#[derive(Clone, ValueEnum)]
enum Format {
    Knowledge,
    Data,
}
#[derive(Parser)]
struct Args {
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    index: PathBuf,
    #[arg(long, value_enum, default_value = "knowledge")]
    format: Format,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Index {
        #[arg(long)]
        full: bool,
        #[arg(long)]
        budget_ms: Option<u64>,
    },
    Search {
        query: String,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        meta: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    Count,
    /// v4 -> v5 dry run (gate G3): copy exhaust into a data vault, verify, index both,
    /// write a registry. `--root` is the knowledge vault; only id-less knowledge files change (their id is written).
    Migrate {
        #[arg(long)]
        exhaust: PathBuf,
        #[arg(long)]
        exhaust_index: PathBuf,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        no_chat_logs: bool,
        /// TSV of `<path relative to --root>\t<id>` from the v4 database, for files with no id.
        #[arg(long)]
        legacy_ids: Option<PathBuf>,
    },
}
fn main() -> Result<()> {
    let args = Args::parse();
    let format = match args.format {
        Format::Knowledge => VaultType::Knowledge,
        Format::Data => VaultType::Data,
    };
    if let Command::Migrate { exhaust, exhaust_index, registry, no_chat_logs, legacy_ids } = &args.command {
        let targets = MigrateTargets { main_index: &args.index, exhaust, exhaust_index, registry };
        return migrate(&args.root, targets, !no_chat_logs, legacy_ids.as_deref());
    }
    let index = VaultIndex::open(&args.root, format, &args.index)?;
    match args.command {
        Command::Index { full, budget_ms } => {
            let options = UpdateOptions {
                full,
                budget: budget_ms.map(std::time::Duration::from_millis),
                ..Default::default()
            };
            println!("{}", serde_json::to_string(&index.update(&options)?)?);
        }
        Command::Search {
            query,
            kind,
            meta,
            limit,
        } => {
            let options = SearchOptions {
                kind,
                meta: meta
                    .map(|raw| serde_json::from_str(&raw))
                    .transpose()?
                    .unwrap_or_default(),
                limit,
                ..Default::default()
            };
            println!(
                "{}",
                serde_json::to_string(&index.search(&query, &options)?)?
            );
        }
        Command::Count => println!("{}", index.count()?),
        Command::Migrate { .. } => unreachable!("handled above"),
    }
    Ok(())
}

struct MigrateTargets<'a> {
    main_index: &'a Path,
    exhaust: &'a Path,
    exhaust_index: &'a Path,
    registry: &'a Path,
}

fn migrate(main: &Path, targets: MigrateTargets, chat_logs: bool, legacy_ids: Option<&Path>) -> Result<()> {
    let MigrateTargets { main_index, exhaust, exhaust_index, registry } = targets;
    std::fs::create_dir_all(exhaust)?;
    let options = MigrateOptions {
        main: main.canonicalize()?,
        exhaust: exhaust.canonicalize()?,
        chat_logs,
        legacy_ids: legacy_ids.map(read_legacy_ids).transpose()?.unwrap_or_default(),
    };
    let report = cv_index::migrate::run(&options)?;
    let full = UpdateOptions { full: true, budget: None, ..Default::default() };
    let data = VaultIndex::open(&options.exhaust, VaultType::Data, exhaust_index)?.update(&full)?;
    // The knowledge index may refuse (duplicate ids, card 46); that is reported, not fatal.
    let knowledge = VaultIndex::open(&options.main, VaultType::Knowledge, main_index)
        .and_then(|index| index.update(&full))
        .map_err(|error| format!("{error:#}"));
    cv_index::migrate::setup::write_registry(registry, &options.main, &options.exhaust)?;
    let out = serde_json::json!({
        "migration": report,
        "exhaust_index": { "indexed": data.indexed, "elapsed_ms": data.elapsed_ms },
        "main_index": knowledge.map(|r| serde_json::json!({"indexed": r.indexed, "invalid_records": r.invalid_records})),
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn read_legacy_ids(path: &Path) -> Result<std::collections::HashMap<String, String>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(rel, id)| (rel.to_string(), id.to_string()))
        .collect())
}
