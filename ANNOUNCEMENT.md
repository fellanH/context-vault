# Announcement Copy — context-mcp v2.1.0

Review and post manually. Adjust links/handles as needed.

---

## Reddit (r/ClaudeAI, r/LocalLLaMA)

**Title:** context-mcp — persistent memory for AI agents via MCP

**Body:**

I built an MCP server that gives AI agents persistent memory across sessions.

**The problem:** Every time you start a new Claude Code / Cursor / Cline session, the agent starts from scratch. All the insights, decisions, and patterns from previous sessions are gone.

**The solution:** context-mcp is a local MCP server that stores your knowledge as plain markdown files and indexes them with FTS5 + vector embeddings for hybrid search. Your agent can save and retrieve context automatically — no cloud, no lock-in, files you own and can git-version.

How it works:
- `save_context` — agent writes insights, decisions, patterns as markdown files
- `get_context` — hybrid full-text + semantic search across everything
- Files live in `~/vault/`, SQLite index at `~/.context-mcp/vault.db`
- Zero config: `npm i -g context-vault && context-mcp setup`

Setup takes under 2 minutes — auto-detects Claude Code, Claude Desktop, Cursor, Windsurf, and Cline, downloads the embedding model upfront (no surprise stalls), seeds your vault with a starter entry, and verifies everything works. You get a working search on your first session.

Built with better-sqlite3, sqlite-vec, and all-MiniLM-L6-v2 for local embeddings. Everything runs locally, no API calls.

GitHub: https://github.com/fellanH/context-mcp
npm: https://www.npmjs.com/package/context-vault

Would love feedback — especially on what kinds of knowledge you'd want your agent to remember.

---

## X / Twitter

**Option A (concise):**

I built context-mcp — persistent memory for AI agents.

Your Claude/Cursor/Cline agent saves insights as markdown, searches them with FTS5 + vector embeddings. Memory that carries across sessions. No cloud, no lock-in.

npm i -g context-vault && context-mcp setup

https://github.com/fellanH/context-mcp

**Option B (hook-first):**

Every new AI coding session starts from scratch. Your agent forgets everything.

context-mcp fixes this — a local MCP server that gives agents persistent memory as plain markdown files with hybrid search.

2-minute setup, works with Claude Code, Cursor, Cline, and more.

https://github.com/fellanH/context-mcp

---

## LinkedIn

**I built an open-source tool that gives AI coding agents persistent memory.**

One thing that's always frustrated me about AI-assisted development: every session starts from zero. The agent doesn't remember the architectural decisions from yesterday, the debugging insights from last week, or the patterns your team has established.

context-mcp solves this. It's a local MCP server that stores knowledge as plain markdown files and indexes them with full-text + semantic search. When your AI agent discovers something worth remembering, it saves it. Next session, it can search for it.

What makes it different:
- **Local-first** — your knowledge stays on your machine as markdown files you own
- **Hybrid search** — combines FTS5 full-text search with vector embeddings (all-MiniLM-L6-v2)
- **2-minute setup** — `npm install`, run `setup`, and it auto-configures your tools, downloads embeddings, and verifies everything
- **Works everywhere** — Claude Code, Claude Desktop, Cursor, Windsurf, Cline

The files live in a vault directory you control. You can edit them, git-version them, or move them anywhere. The SQLite database is just a derived search index — delete it and it rebuilds from your files.

Open source (MIT): https://github.com/fellanH/context-mcp

I'd love to hear how others are handling persistent context in AI-assisted workflows.

#AI #DeveloperTools #OpenSource #MCP #ClaudeCode
