# Context Vault — Agent Operating Protocol

Persistent memory layer for AI agents via MCP. Markdown + SQLite FTS + semantic embeddings.

| Resource                | URL                                                |
| ----------------------- | -------------------------------------------------- |
| Repo                    | https://github.com/fellanH/context-vault           |
| App                     | https://app.context-vault.com                      |
| Marketing               | https://contextvault.dev                           |
| API                     | https://api.context-vault.com                      |
| context-vault-app       | https://github.com/fellanH/context-vault-app       |
| context-vault-marketing | https://github.com/fellanH/context-vault-marketing |
| context-vault-extension | https://github.com/fellanH/context-vault-extension |

## Package Architecture (post-split)

| Package  | Runtime                        | Storage                                                        | Notes                                                     |
| -------- | ------------------------------ | -------------------------------------------------------------- | --------------------------------------------------------- |
| `core`   | shared library                 | —                                                              | Capture, search (SQLite+embeddings), MCP tool definitions |
| `local`  | local MCP server (stdio) + CLI | `~/.context-mcp/vault.db` + `~/vault/`                         | Bundles app UI for `context-vault ui`                     |
| `hosted` | cloud MCP server (HTTP)        | Turso/LibSQL (meta) + per-user SQLite on Fly.io `/data/users/` | Includes auth, Stripe billing, S3 backup                  |

Extracted to separate repos:

- `app` → https://github.com/fellanH/context-vault-app (Vercel, app.context-vault.com)
- `marketing` → https://github.com/fellanH/context-vault-marketing (Vercel, context-vault.com)
- `extension` → https://github.com/fellanH/context-vault-extension (Chrome Web Store)

## Tone

Technical, honest about trade-offs. No hype, no fabricated metrics. Developer-to-developer.

---

## Session Protocol

### On every session start — before anything else

**Step 0: Load north star**
Read `NORTH-STAR.md`. Human-authored product direction. Never modify it. Hold it for the session.

**Step 1: Derive live state — run these, do not trust docs**

```bash
git status && git log --oneline -5          # where are we, what's been done
npm view context-vault version              # what's live on npm
gh run list --workflow=deploy.yml --limit=3 # recent deploys
```

Read `BACKLOG.md` for priorities. Read `FEEDBACK.md` and `INBOX.md` for pending human input.

Surface a brief summary: what's recently shipped, highest-priority next item.
If not on main — flag it and ask why before proceeding.

**Step 2: Receive session goal**
Human declares: `Session goal: [one sentence]`
If not provided, ask for it before proceeding.

**Step 3: Self-governance check — before writing any code**

1. Does this conflict with any other in-progress work? (git status)
2. Does this serve the product direction? (NORTH-STAR.md)
3. **Go** → pitch approach, wait for approval, proceed
4. **Hold** → name the conflict, wait for human decision

---

## Input Commands (handle at any point)

**`capture: [idea]`** → append to `INBOX.md`: `YYYY-MM-DD — [idea]`
Confirm in one line. Return to whatever was in progress.

**`feedback: [observation]`** → append to `FEEDBACK.md`: `YYYY-MM-DD [UX/DEV] — [observation]`
`[UX]` = user experience. `[DEV]` = developer experience. Confirm and return.

If 3+ short generative prompts in a row: shift to capture mode —
_"Looks like you're in capture mode — logging these, pausing implementation. Keep going."_

`chaos mode on` → everything to INBOX.md until `chaos mode off`

---

## The Work Cycle

### Default: work on main

Commit directly to main. Push triggers CI. CI success triggers deploy. No PRs, no branches.

### Branch only when:

- Two or more agents working in parallel (they'd conflict on main)
- Experimental work you might abandon

Branch naming when needed: `feat/<scope>-<slug>` · `fix/<scope>-<slug>` · `chore/<slug>`
Scope: `marketing · hosted · cli · extension · core · infra`

### Work

- Implement, test, conventional commits (`feat:`, `fix:`, `chore:`)
- Reference issues in commits: `Fixes #N`
- Code self-documents. No inline comments unless logic is genuinely non-obvious.

### Ship

`git push origin main` → CI runs → deploy pipeline triggers automatically

### Update

- Move BACKLOG.md item: `Now` → `Done`
- File GitHub issues for newly discovered work
- No docs for anything derivable from code

### Session wrap (triggered by `Session wrap.`)

- Run state derivation — confirm what actually shipped
- Correct BACKLOG.md if it drifted
- Dump surfaced ideas/feedback to INBOX.md or FEEDBACK.md
- Save session review: `save_context` (kind: `session-review`, tags: `context-vault, retro`)
- Tell human what's ready for next session

---

## Deploy Pipeline

Push to main → CI (`test` + `check-constants`) → if backend changed → deploy production → health check.
Branch protection requires both CI jobs to pass before merge.

| Package         | Trigger                            | Target                      |
| --------------- | ---------------------------------- | --------------------------- |
| packages/local  | git tag `v*`                       | npm                         |
| packages/hosted | main push (hosted or core changed) | Fly.io production           |
| packages/core   | no direct deploy                   | bundled into local + hosted |

**npm release protocol:**

1. `npm version patch|minor|major -w packages/local` (also bump root + core to match)
2. `git push origin main && git push --tags`
3. CI runs → publish.yml picks up `v*` tag → tests → publishes to npm → creates GitHub release

**Versioning:** bump root + core + local together.
`packages/hosted` is 0.x — never bump with the others.

---

## Triage (agent-led, human-invoked)

Human prompt: `Triage. No code.`

Agent:

1. Derive state from git — what actually shipped recently
2. Read FEEDBACK.md — translate entries into scored backlog items
3. Read INBOX.md — ICE score each, move to Next/Later/discard
4. Re-score Next based on feedback signals
5. Correct BACKLOG.md Now to match active work
6. Tell human what to pull into Now

ICE = Impact(1–5) × Confidence(1–5) × Ease(1–5). Feedback drives Impact. Human never scores manually.

---

## Agent Boundaries

| Agent              | Allowed paths                                | Notes                     |
| ------------------ | -------------------------------------------- | ------------------------- |
| cv-site-dev        | context-vault-marketing repo                 | Moved to separate repo    |
| cv-content-writer  | context-vault-marketing repo (content/posts) | Moved to separate repo    |
| cv-campaigns       | context-vault-marketing repo (assets)        | Moved to separate repo    |
| cv-sales-ops       | docs/gtm/ pipeline files                     |                           |
| cv-release-manager | version files, CHANGELOG, git tags           | Clean main only           |
| cv-test-runner     | read-only — no commits                       |                           |
| Claude CLI         | any package                                  | Default for all code work |

---

## Sprint Tracking

BACKLOG.md `Now` reflects active work. Agents correct drift when they see it.

| Event          | Action                 |
| -------------- | ---------------------- |
| Work started   | Move item to Now       |
| Pushed to main | Move item to Done      |
| Work abandoned | Move item back to Next |

Hard cap: 3 items in `Now`.

---

## GTM Workflow

| Step   | Engineering  | GTM                             |
| ------ | ------------ | ------------------------------- |
| Orient | BACKLOG.md   | GTM-BACKLOG.md + GTM-CONTEXT.md |
| Ship   | push to main | post/publish + verify live      |
| Update | BACKLOG.md   | GTM-BACKLOG.md + weekly-log.md  |

---

## Issue Labels

`bug` · `feature` · `enhancement` · `dx` · `infra` · `gtm` · `user-request` · `P0`–`P3`
