//! Clap arg definitions for the `context-vault` binary. Kept separate from `main.rs`
//! (which just parses and dispatches) purely to stay under this repo's file-length
//! ratchet -- no behavioral reason for the split.

#[cfg(feature = "dev-tools")]
use std::path::PathBuf;

#[cfg(feature = "dev-tools")]
use context_vault_core::bench_harness;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "context-vault")]
#[command(about = "Context Vault Rust CLI — setup, MCP, and engine commands")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Register context-vault MCP server with AI tools.
    Setup {
        /// Target client: claude-desktop, claude-code, cursor, windsurf, cline, roo-code, or all (default).
        #[arg(long)]
        client: Option<String>,
        /// Print JSON snippet and paths without writing.
        #[arg(long)]
        print: bool,
        /// Show what would be written without writing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Deterministic bench harness; prints JSON (includes bench-meta). Dev build only
    /// (`--features dev-tools`) — not part of the shipped binary's command surface.
    #[cfg(feature = "dev-tools")]
    Bench {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long, value_enum, default_value_t = bench_harness::DiskMode::A)]
        disk_mode: bench_harness::DiskMode,
        #[arg(long, value_enum, default_value_t = bench_harness::WorkloadMix::ReadHeavy)]
        workload: bench_harness::WorkloadMix,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long)]
        corpus_root: Option<PathBuf>,
        #[arg(long, default_value_t = 128)]
        corpus_entries: u32,
        /// Read-only copy of a real vault.db (omni slice dry run). Skips fixture seed; FTS token from first row.
        #[arg(long)]
        vault_db: Option<PathBuf>,
        /// Write the same JSON payload to this path (parent dirs created).
        #[arg(long)]
        archive_json: Option<PathBuf>,
    },
    /// MCP stdio server: serves the vault engine in-process over JSON-RPC on stdin/stdout.
    Mcp,
    /// Rebuild the SQLite index from the markdown vault (FTS + rows).
    Reindex {
        /// Re-parse every file instead of only files changed since the last reindex.
        #[arg(long)]
        full: bool,
        /// v5: print what a full run would read and its estimated time, then stop.
        #[arg(long)]
        plan: bool,
        /// Print the plan as JSON.
        #[arg(long)]
        json: bool,
        /// v5: refuse an unattended full run estimated above this many seconds.
        #[arg(long, default_value_t = 30)]
        max_seconds: u64,
    },
    /// v4 -> v5: copy exhaust into a data vault (originals stay), verify, index, write vaults.toml.
    Migrate {
        /// Data vault dir (default: `<vault>-exhaust` beside the vault).
        #[arg(long)]
        exhaust: Option<std::path::PathBuf>,
        /// Leave events/chat-logs transcripts out.
        #[arg(long)]
        no_chat_logs: bool,
    },
    /// Print vault status: entry counts, health, resolved paths.
    Status,
    /// Diagnose the install: check db/vault paths and engine health.
    Doctor,
    /// Manage vaults: vaults.toml on v5, ~/.context-mcp/vaults.json before migrating.
    Vault {
        #[command(subcommand)]
        command: VaultCommands,
    },
    /// Inspect or scaffold the per-vault taxonomy (vault.toml): kind -> category/tier/folder.
    Schema {
        #[command(subcommand)]
        command: SchemaCommands,
    },
    /// Move a file or folder within the vault (mv-like; contents of a directory land
    /// flat under dest, not nested). Never overwrites. Fast: no taxonomy inference,
    /// no reindex -- just a filesystem move plus a targeted file_path DB sync.
    Move {
        source: String,
        dest: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Save an entry to the vault.
    Save {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        tags: Option<String>,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        meta: Option<String>,
        #[arg(long)]
        identity_key: Option<String>,
    },
    /// Retrieve an entry by ID.
    Get { id: String },
    /// Hybrid full-text + semantic search.
    Search {
        query: String,
        #[arg(long)]
        limit: Option<usize>,
        /// text: human-readable table (default). json: full scored entries.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Recall lightweight hints from a raw signal (prompt text, error message, or file
    /// path). Wraps the MCP `recall` tool so shells and hooks can surface related vault
    /// entries without an MCP client.
    Recall {
        /// Raw signal: prompt text, an error message, or a file path.
        signal: String,
        /// Max hints to return (default 3).
        #[arg(long)]
        limit: Option<usize>,
        /// Project bucket to scope hints to (default: current directory's name).
        #[arg(long)]
        bucket: Option<String>,
        /// Search the whole vault, ignoring project scoping.
        #[arg(long, conflicts_with = "bucket")]
        global: bool,
    },
    /// Delete an entry by ID.
    Delete { id: String },
    /// List all entries with optional filters.
    List {
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Print vault statistics as JSON.
    Stats,
    /// Export one kind's entries from the markdown files: id, title, created, updated,
    /// the kind's declared fields, then other meta as JSON.
    Export {
        #[arg(long)]
        kind: String,
        #[arg(long, default_value = "csv")]
        format: String,
    },
    /// List entries older than their kind's vault.toml `fresh_days` window.
    Stale {
        #[arg(long)]
        kind: Option<String>,
    },
    /// Mark an entry as checked and still true (sets meta.verified_at to now).
    Verify { id: String },
    /// Ingest from a URL.
    Ingest { url: String },
    /// Ingest from a project directory.
    IngestProject { path: String },
    /// Archive entries by criteria.
    Archive {
        #[arg(long)]
        days: Option<u32>,
    },
    /// Restore an archived entry.
    Restore { id: String },
    /// Generate summaries for entries.
    Summarize,
    /// Analyze and apply tier reassignments.
    Tier,
    /// Assemble context from related entries.
    Assemble { ids: Vec<String> },
    /// PostToolUse hook target. No-op: post-tool-call hooks were removed upstream in
    /// v3 (the currently-published npm CLI's handler is also empty) — this subcommand
    /// exists only so the global `PostToolUse` hook (`context-vault post-tool-call`)
    /// doesn't error when this binary is the one on PATH.
    PostToolCall,
    /// SessionEnd hook target: reads the hook payload from stdin, parses the
    /// transcript, captures a session summary + auto-extracted insights, and
    /// writes the card-23 project session log. Never fails loudly -- a broken
    /// hook must not block session end.
    SessionEnd {
        /// Batch mode: process every `.jsonl` transcript in this directory
        /// instead of reading one hook payload from stdin (card 23 backfill --
        /// same writer, not a second one). Requires `--cwd-root`.
        #[arg(long)]
        backfill: Option<String>,
        /// The current project directory to write session logs into during
        /// `--backfill` (explicit, not derived from the transcript's own
        /// recorded cwd, since that may be a stale pre-reorg path).
        #[arg(long)]
        cwd_root: Option<String>,
    },
    /// UserPromptSubmit hook target: reads the hook payload (incl. `prompt`) from
    /// stdin, runs it through the same recall logic as the MCP `recall` tool, and
    /// emits any hints as `hookSpecificOutput.additionalContext` JSON on stdout.
    /// Never fails loudly -- prints nothing (not even a stray newline) on any
    /// error or when there's nothing worth surfacing, and always exits 0.
    RecallHint,
    /// Internal: single tool invocation (stdin JSON when --stdin-json). Dev build only.
    #[cfg(feature = "dev-tools")]
    #[command(hide = true)]
    Invoke {
        #[arg(long)]
        stdin_json: bool,
    },
}

#[derive(Subcommand)]
pub enum VaultCommands {
    /// List registered vaults (default/active marked with *).
    List,
    /// Switch the active (v5: default) vault by name.
    Use {
        name: String,
    },
    /// v5: register another vault in vaults.toml.
    Add {
        name: String,
        dir: std::path::PathBuf,
        /// A data vault (append-only segments) instead of a knowledge vault.
        #[arg(long)]
        data: bool,
        /// Search it only when named, not by default.
        #[arg(long)]
        by_name_only: bool,
    },
    /// v5: forget a vault (its files and index stay on disk).
    Remove {
        name: String,
    },
}

#[derive(Subcommand)]
pub enum SchemaCommands {
    /// Write the built-in taxonomy as an editable vault.toml (won't overwrite an existing one).
    Init,
    /// Print the effective taxonomy (vault.toml if present, else built-in defaults).
    Show,
    /// Move existing files to match the current vault.toml, then reindex. Run this
    /// right after editing vault.toml, before further saves, to avoid duplicate
    /// entries for identity-based kinds (see specs/vault-taxonomy.md).
    Apply {
        /// Print the move/skip/unchanged plan without touching disk or reindexing.
        #[arg(long)]
        dry_run: bool,
    },
}
