# context-vault

[![npm version](https://img.shields.io/npm/v/context-vault)](https://www.npmjs.com/package/context-vault)
[![npm downloads](https://img.shields.io/npm/dm/context-vault)](https://www.npmjs.com/package/context-vault)
[![license](https://img.shields.io/npm/l/context-vault)](./LICENSE)
[![node](https://img.shields.io/node/v/context-vault)](https://nodejs.org)

Persistent memory for AI agents — saves and searches knowledge across sessions.

<p align="center">
  <img src="https://github.com/fellanH/context-vault/raw/main/demo.gif" alt="context-vault demo — Claude Code and Cursor using the knowledge vault" width="800">
</p>

## Quick Start

```bash
npm install -g context-vault
context-vault setup
```

Setup auto-detects your tools (Claude Code, Codex, Claude Desktop, Cursor, Windsurf, Antigravity, Cline), downloads the embedding model, seeds your vault with a starter entry, and verifies everything works. Then open your AI tool and try:

> "Search my vault for getting started"

For hosted MCP setup (Claude Code, Cursor, GPT Actions), see [connect-in-2-minutes](https://github.com/fellanH/context-vault/blob/main/docs/distribution/connect-in-2-minutes.md).

> **Note:** `context-mcp` still works as a CLI alias but `context-vault` is the primary command.

## What It Does

- **Save** insights, decisions, patterns, and any custom knowledge kind from AI sessions
- **Search** with hybrid full-text + semantic similarity, ranked by relevance and recency
- **Own your data** — plain markdown files in folders you control, git-versioned, human-editable

## AI Tool Examples

Tell your AI agent any of these:

- **"Search my vault for React hooks"** → hybrid full-text + semantic search
- **"Save an insight: always use useCallback for event handlers"** → creates a new entry
- **"List my recent decisions"** → browse entries filtered by kind
- **"Show my vault status"** → diagnostics and health check
- **"Delete entry 01HXYZ..."** → remove by ID

## Common Workflows

### Save and Retrieve

```
You: "Save an insight: React Query's staleTime defaults to 0"
AI:  ✓ Saved insight → knowledge/insights/react-querys-staletime-defaults.md

You: "Search my vault for React Query"
AI:  [Returns the saved insight with full content]
```

### Build a Project Knowledge Base

```
You: "Save a decision: we chose SQLite over Postgres for the local-first architecture"
You: "Save a pattern: all API handlers follow the try/catch/respond pattern in src/api/"
You: "What decisions have we made about the database?"
AI:  [Returns relevant decisions ranked by relevance]
```

### Track Contacts and Entities

```
You: "Save a contact for Alice (alice@example.com) — lead developer on Project X"
You: "Search my vault for Alice"
AI:  [Returns the contact entry]
```

### Session Summaries

```
You: "Save a session summary: debugged auth token refresh, fixed race condition in useAuth hook"
You: "What did I work on last week?"
AI:  [Returns recent session entries]
```

## Knowledge Organization

### Folders and Kinds

Entries are organized into three categories — **knowledge** (enduring insights, decisions, patterns), **entity** (contacts, projects, tools), and **event** (sessions, conversations, logs). See [DATA_CATEGORIES](https://github.com/fellanH/context-vault/blob/main/docs/DATA_CATEGORIES.md) for the full category system, kind mappings, and write semantics.

Each top-level subdirectory in the vault maps to a `kind` value. The directory name is depluralized:

```
knowledge/insights/    →  kind: "insight"
knowledge/decisions/   →  kind: "decision"
knowledge/patterns/    →  kind: "pattern"
knowledge/references/  →  kind: "reference"
```

Within each kind directory, nested subfolders provide human-browsable organization. The subfolder path is stored in `meta.folder`:

```
ON DISK                                    IN DB (vault table)
knowledge/insights/                        kind: "insight", meta.folder: null
  flat-file.md
knowledge/insights/react/hooks/            kind: "insight", meta.folder: "react/hooks"
  use-query-gotcha.md
```

Tags are semantic (what the content is about). Folder structure is organizational (where it lives). These are separate concerns.

### File Format

All knowledge files use YAML frontmatter:

```markdown
---
id: 01HXYZ...
tags: ["react", "performance"]
source: claude-code
created: 2026-02-17T12:00:00Z
---

React Query's staleTime defaults to 0 — set it explicitly or every mount triggers a refetch.
```

Standard keys: `id`, `tags`, `source`, `created`. Any extra frontmatter keys (`type`, `status`, `language`, `folder`, etc.) are stored in a `meta` JSON column automatically.

### Custom Kinds

No code changes required:

1. `mkdir ~/vault/knowledge/references/`
2. Add `.md` files with YAML frontmatter
3. The next session auto-indexes them

The kind name comes from the directory: `references/` → kind `reference`.

## Install

### npm (Recommended)

```bash
npm install -g context-vault
context-vault setup
```

The `setup` command auto-detects installed tools (Claude Code, Codex, Claude Desktop, Cursor, Windsurf, Antigravity, Cline), lets you pick which to configure, and writes the correct MCP config for each. Existing configs are preserved — only the `context-vault` entry is added or updated.

### Manual Configuration

If you prefer manual setup, add to your tool's MCP config. Pass `--vault-dir` to point at your vault folder (omit it to use the default `~/vault/`).

**npm install** (portable — survives upgrades):

```json
{
  "mcpServers": {
    "context-vault": {
      "command": "context-vault",
      "args": ["serve", "--vault-dir", "/path/to/vault"]
    }
  }
}
```

You can also pass config via environment variables in the MCP config block:

```json
{
  "mcpServers": {
    "context-vault": {
      "command": "context-vault",
      "args": ["serve"],
      "env": {
        "CONTEXT_VAULT_VAULT_DIR": "/path/to/vault"
      }
    }
  }
}
```

### How the Server Runs

The server is an MCP (Model Context Protocol) process — you don't start or stop it manually. Your AI client (Claude Code, Codex, Cursor, Windsurf, Cline, etc.) spawns it automatically as a child process when a session begins, based on the `mcpServers` config above. The server communicates over stdio and lives for the duration of the session. When the session ends, the client terminates the process and SQLite cleans up its WAL files.

This means:

- **No daemon, no port, no background service.** The server only runs while your AI client is active.
- **Multiple sessions** can run separate server instances concurrently — SQLite WAL mode handles concurrent access safely.
- **Embedding model** is downloaded during `setup` (~22MB, all-MiniLM-L6-v2). If setup was skipped, it downloads on first use.
- **Auto-reindex** on first tool call per session ensures the search index is always in sync with your files on disk. No manual reindex needed.

## Web Dashboard

The `context-vault ui` command launches a web dashboard for browsing, searching, and managing your vault entries.

## How It Works

```
YOUR FILES (source of truth)         SEARCH INDEX (derived)
~/vault/                         ~/.context-mcp/vault.db
├── knowledge/                       ┌───────────────────────────────┐
│   ├── insights/                    │ vault table                   │
│   │   ├── react-query-caching.md   │   kind: insight               │
│   │   └── react/hooks/             │   meta.folder: "react/hooks"  │
│   │       └── use-query-gotcha.md  │   kind: decision              │
│   ├── decisions/                   │   kind: pattern               │
│   │   └── use-sqlite-over-pg.md    │   kind: <any custom>          │
│   └── patterns/                    │ + FTS5 full-text              │
│       └── api-error-handler.md     │ + vec0 embeddings             │
├── entities/                        └───────────────────────────────┘
└── events/
Human-editable, git-versioned        Fast hybrid search, RAG-ready
You own these files                  Rebuilt from files anytime
```

The SQLite database is stored at `~/.context-mcp/vault.db` by default (configurable via `--db-path`, `CONTEXT_VAULT_DB_PATH`, or `config.json`). It contains FTS5 full-text indexes and sqlite-vec embeddings (384-dim float32, all-MiniLM-L6-v2). The database is a derived index — delete it and the server rebuilds it automatically on next session.

Requires **Node.js 20** or later.

## Troubleshooting

### Native module build failures

`better-sqlite3` and `sqlite-vec` include native C code compiled for your platform. If install fails:

```bash
npm rebuild better-sqlite3 sqlite-vec
```

On Apple Silicon Macs, ensure you're running a native ARM Node.js (not Rosetta). Check with `node -p process.arch` — it should say `arm64`.

### Vault directory not found

If `context_status` or `get_context` reports the vault directory doesn't exist:

```bash
context-vault status    # Shows resolved paths
mkdir -p ~/vault      # Create the default vault directory
```

Or re-run `context-vault setup` to reconfigure.

### Embedding model download

The embedding model (all-MiniLM-L6-v2, ~22MB) is normally downloaded during `context-vault setup`. If setup was skipped or the cache was cleared, it downloads automatically on first use. If it hangs, check your network or proxy settings.

### Stale search index

If search results seem outdated or missing:

```bash
context-vault reindex
```

This rebuilds the entire index from your vault files. Auto-reindex runs on every session start, but manual reindex can help diagnose issues.

### Config path debugging

```bash
context-vault status
```

Shows all resolved paths (vault dir, data dir, DB path, config file) and where each was resolved from (defaults, config file, env, or CLI args).

## Dependencies

| Package                     | Purpose                                        |
| --------------------------- | ---------------------------------------------- |
| `@modelcontextprotocol/sdk` | MCP protocol (McpServer, StdioServerTransport) |
| `better-sqlite3`            | SQLite driver                                  |
| `sqlite-vec`                | Vector search (384-dim float32)                |
| `@huggingface/transformers` | Local embeddings (all-MiniLM-L6-v2, ~22MB)     |

## License

MIT
