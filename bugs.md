# Bugs, Improvements & Roadmap

**Last updated:** 2026-02-20

---

## Contents

- [Active Bugs](#active-bugs)
- [In Progress](#in-progress)
- [Backlog](#backlog)
- [Future](#future)
- [Completed](#completed)

---

## Active Bugs

None known.

---

## In Progress

_Nothing in progress._

---

## Backlog

_Empty — all items shipped._

---

## Future

### Someday

**Pricing & free tier**

- Review and refine pricing tiers and free user model over time.

**Multi-source URL ingestion pipelines**

- Different pipelines per source type (video transcripts, social posts, PDFs).
- Current URL ingestion handles HTML pages; extend to more content types.

---

## Completed

| Item | Status | Resolution | Date |
|------|--------|------------|------|
| API key creation fails in local mode | Fixed | Local mode disables hosted API key create/delete; shows local-mode guidance instead of failing. | 2026-02 |
| Storage card number overflow | Fixed | Storage values normalized and displayed with concise precision (up to 2 decimals). | 2026-02 |
| Search result click does nothing | Fixed | Search results open inline entry inspector; Enter/Space keyboard activation supported. | 2026-02 |
| Drawer layout issues | Fixed | Entry drawer has improved spacing; metadata uses compact responsive grid. | 2026-02 |
| Getting started card validity | Improved | Checklist completion derived from real signals (auth mode, API key, key activity, entries). | 2026-02 |
| MCP server status visibility | Added | Top bar shows mode + connection: `Local/Hosted` + `Connected/Degraded/Disconnected`. | 2026-02 |
| Dashboard quick setup bar | Added | When steps incomplete: direct actions for Create API key, Copy MCP config, Add first entry; "Show full checklist" when dismissed. | 2026-02 |
| First-entry step links | Added | Checklist now links to Add entry and Import data when first-entry incomplete. | 2026-02 |
| Dashboard & onboarding redesign | Done | Hero step cards with icons, action buttons, completion tracking; "Setup complete" badge with reset. | 2026-02 |
| Hosted UX improvements | Done | CLI `connect --key` command, native folder picker (Browse button), bulk import endpoint, local-to-hosted upload prompt. | 2026-02 |
| Google OAuth sign-in | Done | Google OAuth backend + frontend; auto API key generation; AuthCallback page; email registration kept as fallback. | 2026-02 |
| Codex & Antigravity CLI support | Done | Added Codex (OpenAI) and Antigravity (Google) to `context-mcp setup` detection, configuration, and uninstall. | 2026-02 |
| Connect command shows `--key YOUR_API_KEY` in local mode | Fixed | Local mode now shows `npx context-vault connect` without `--key` flag; hosted mode keeps the placeholder. | 2026-02 |
| Billing page fails on localhost | Fixed | Local mode billing page shows informational card instead of broken upgrade buttons; links to create cloud account. | 2026-02 |
| Extension install link missing from local mode | Fixed | Added `install-extension` onboarding step to local mode; added persistent "Browser Extension" link in sidebar. | 2026-02 |
| Extension UI hardcoded dark theme | Fixed | Extension theme.css now defaults to light mode with `prefers-color-scheme: dark` media query, matching app design tokens. | 2026-02 |
| Data import flexibility | Done | Multi-format import (markdown, CSV/TSV, JSON, text, directories). CLI `import`, REST bulk/file endpoints, ChatGPT export support. | 2026-02 |
| Local-mode API parity | Done | Added export (JSON/CSV), CORS preflight, bulk import endpoints to local server. | 2026-02 |
| External URL ingestion | Done | `ingest_url` MCP tool, CLI `ingest` command, REST endpoints on local + hosted. HTML→markdown with content extraction. | 2026-02 |
| Export & data ownership | Done | `context-vault export` CLI + `GET /api/vault/export` on both local and hosted. JSON and CSV formats. | 2026-02 |
| Account linking & sync | Done | `context-vault link/sync` CLI, bidirectional additive-only sync, manifest-based diffing, dashboard Sync settings page. | 2026-02 |
