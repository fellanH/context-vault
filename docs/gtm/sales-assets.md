# Sales Assets

Collateral for founder-led sales. Status: `not-started` → `in-progress` → `review` → `done`

---

## 1. Solution Brief

One-page PDF following pain → promise → proof format.

| Asset | Status | Target | Location | Notes |
|-------|--------|--------|----------|-------|
| Solution brief PDF | not-started | W4 | `docs/gtm/assets/solution-brief.pdf` | Pain: stateless AI. Promise: persistent memory in minutes. Proof: open-core + hosted MCP. |

**Outline:**
- Pain: AI sessions restart from zero. Decisions, patterns, and context vanish.
- Promise: Context Vault gives your AI tools persistent memory through MCP — set up in minutes, own your data forever.
- Proof: open-source local vault, hosted MCP endpoint, markdown portability, hybrid search.
- CTA: Start free at contextvault.dev

---

## 2. Demo Scripts

Each demo has a script file in `docs/gtm/demos/`. Scripts cover setup, key moments, and expected output.

| # | Demo | Status | Target | Script Location | Duration |
|---|------|--------|--------|-----------------|----------|
| 1 | CLI local setup | not-started | W4 | `docs/gtm/demos/cli-local.md` | 2-3 min |
| 2 | Hosted MCP endpoint | not-started | W5 | `docs/gtm/demos/hosted-mcp.md` | 2-3 min |
| 3 | Browser extension inject | not-started | W6 | `docs/gtm/demos/extension-inject.md` | 2-3 min |

---

## 3. Campaign Materials

### Campaign A: "Ship MCP memory in 5 minutes" (W5)

Target: developers already using MCP-compatible tools who want persistent context.

| Asset | Status | Notes |
|-------|--------|-------|
| X thread (5 tweets) | not-started | Hook: "Your AI forgets everything. Here's how to fix it in 5 min." |
| Reddit post (r/ClaudeAI, r/cursor) | not-started | Integration guide format |
| HN Show post | not-started | Technical angle: local-first + hybrid search |
| Landing page variant | not-started | UTM-tagged `/` with campaign messaging |

### Campaign B: "Local to hosted without lock-in" (W7)

Target: privacy-conscious developers evaluating hosted options.

| Asset | Status | Notes |
|-------|--------|-------|
| X thread (5 tweets) | not-started | Hook: "I moved 500 vault entries to hosted in 2 minutes. No vendor lock-in." |
| Blog companion post | not-started | Links to post #12 (local-first vs hosted) |
| Comparison one-pager | not-started | Side-by-side local vs hosted feature matrix |

### Campaign C: "Inject vault context into ChatGPT/Claude/Gemini" (W9)

Target: users of multiple AI tools who want cross-platform memory.

| Asset | Status | Notes |
|-------|--------|-------|
| X thread (5 tweets) | not-started | Hook: "Same memory across Claude, ChatGPT, and Cursor." |
| Demo video (extension) | not-started | Show inject flow across 3 different AI UIs |
| GPT Actions integration post | not-started | Links to post #6 (MCP + GPT Actions) |

---

## 4. Objection Handling Cheatsheet

Quick-reference for founder conversations. Keep in pipeline notes or open during calls.

| Objection | One-liner | Longer response | Status |
|-----------|-----------|-----------------|--------|
| "I already use notes/Notion" | "Notes aren't retrieval-ready for agents. CV uses hybrid search so your AI finds what it needs." | Explain FTS + semantic search, auto-indexing, MCP protocol vs manual copy-paste. | not-started |
| "What about privacy?" | "Your data stays in your vault. Hosted accounts are isolated by API key. You can export anytime." | Local-first architecture, no training on user data, markdown portability. | not-started |
| "Will I get locked in?" | "Everything is markdown files. Export, move, or self-host whenever you want." | Open-source core, standard MCP protocol, no proprietary formats. | not-started |
| "Seems complex to set up" | "One MCP endpoint plus copy-paste config. Under 5 minutes." | Point to CLI setup post and hosted MCP demo. | not-started |
| "Why not just use .cursorrules / CLAUDE.md?" | "Those are static. CV gives semantic search, tagging, and retrieval across sessions." | Explain growth beyond single files, cross-project memory, hybrid search. | not-started |
| "I'll build my own" | "You could! CV saves you weeks and gives you hybrid search + hosted MCP out of the box." | Acknowledge it's possible, emphasize time-to-value and maintained infrastructure. | not-started |
