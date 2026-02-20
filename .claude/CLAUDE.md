# Context Vault — Project Context

## Product

- **Name:** Context Vault
- **What it is:** Persistent memory layer for AI agents via MCP (Model Context Protocol)
- **Core tech:** markdown files + SQLite FTS + semantic embeddings, served over MCP
- **Repo:** `github.com/fellanH/context-vault`
- **Marketing site:** contextvault.dev (SPA in `packages/marketing/`)
- **App:** context-vault.com (SPA in `packages/app/`)

## Key URLs

| Resource | URL |
|----------|-----|
| GitHub repo | `https://github.com/fellanH/context-vault` |
| Marketing site | `https://contextvault.dev` |
| App | `https://context-vault.com` |
| Docs quickstart | `https://github.com/fellanH/context-vault/blob/main/docs/distribution/connect-in-2-minutes.md` |

## Tone

- Technical, helpful, honest about trade-offs
- No hype, no fabricated metrics, no fake testimonials
- Show real commands, real output, real workflows
- Speak as a developer building tools for other developers

## Session Protocol

Every Claude Code working session follows this workflow:

### 1. Orient
- Read `BACKLOG.md` to understand current engineering priorities
- For GTM sessions: also read `docs/gtm/GTM-BACKLOG.md` and `docs/gtm/GTM-CONTEXT.md`
- Check `Now` section for active work items (in whichever backlog applies)
- Query context vault for recent sessions and decisions: `get_context` with tags `context-vault`

### 2. Pick
- Work on an item from `Now`, or triage if the user requests it
- If `Now` is empty, pull the highest-ICE item from `Next`
- For GTM work: pick from `docs/gtm/GTM-BACKLOG.md` instead

### 3. Pitch
Present the plan for user approval **before writing any code**. Scale depth to complexity:

**For non-trivial work** (multi-file, design choices, or multi-item sprints):
- **Why this, why now** — the business/technical case, not just backlog position
- **Options** — 2-3 approaches with trade-offs (recommend one)
- **Scope & risk** — files affected, what could break, dependencies between items
- If batching multiple items: pitch the batch as a release — why these together, ordering, parallel vs sequential

**For trivial work** (single-file fix, typo, obvious change):
- One-line summary: "Fixing X because Y, no alternatives, proceeding"

Wait for explicit user approval before moving to Branch.

### 4. Branch
- **Always** create a branch: `feat/<name>`, `fix/<name>`, or `chore/<name>`
- No direct commits to `main` — branch protection enforces this (see #21)
- GTM tasks that produce no code changes skip this step

### 5. Work
- Implement, test, commit with conventional commit messages
- Reference the GitHub issue: `Fixes #N` in commit messages

### 6. Ship
- Push branch, wait for CI (`test-and-build` must pass), then merge
- Create a PR with `Fixes #N` to auto-close the issue on merge
- Squash merge via `gh pr merge --squash --admin` (admin bypass skips review requirement for solo work, but CI must have passed)
- If CI fails: fix on the branch, push again, wait for green, then merge

### 7. Update
- Update `BACKLOG.md`: move completed items, add new signals, adjust priorities
- If new work was discovered during the session, file GitHub issues

### 8. Review
End-of-session retrospective. Scale depth to session complexity:

**Format:**
- **Shipped** — what got done (note any diff from the pitch plan)
- **Went well** — patterns worth repeating
- **Friction** — what slowed us down or went wrong
- **Learned** — new codebase knowledge, tool insights, process observations
- **Actions** — concrete follow-ups: update CLAUDE.md rules, add memory entries, file issues

**Persistence:**
- Save the review to context vault (`save_context` with kind: `session-review`, tags: `context-vault, retro`)
- If a lesson is durable (applies beyond this session), also add it to memory files or CLAUDE.md
- If friction points are actionable, file GitHub issues or add to backlog

### Branch naming
| Prefix | Use |
|--------|-----|
| `feat/` | New features |
| `fix/` | Bug fixes |
| `chore/` | Infra, deps, cleanup |

### Issue labels
| Label | Purpose |
|-------|---------|
| `bug` | Something broken |
| `feature` | New capability |
| `enhancement` | Improvement to existing feature |
| `dx` | Developer experience (setup, docs, onboarding) |
| `infra` | CI/CD, deployment, monitoring |
| `gtm` | Marketing, sales, content |
| `user-request` | Directly from a user |
| `P0-critical` | Must fix before next release |
| `P1-high` | Next release |
| `P2-medium` | Soon |
| `P3-low` | Eventually |

### ICE scoring (for ordering `Next` items)
- **Impact** (1-5): How many users affected? How much does it move revenue/adoption?
- **Confidence** (1-5): How sure is this the right thing? (User signal = high, gut = low)
- **Ease** (1-5): How fast can it ship? (1 session = 5, multi-day = 1)
- Score = I × C × E. Highest goes to top of `Next`.

### Weekly triage
- Review vault `feedback` entries and new GitHub issues
- Add community signals (Reddit, X, HN) to `Signals` section
- Re-score `Next` items if priorities shifted
- Pull top items into `Now` (max 3)

### GTM Task Workflow

GTM tasks follow the same session protocol but differ in execution:

| Step | Engineering | GTM |
|------|------------|-----|
| Orient | Read `BACKLOG.md` | Read `GTM-BACKLOG.md` + `GTM-CONTEXT.md` |
| Pick | Pull from Now/Next | Same |
| Pitch | Plan before code | Plan before action (what, where, expected outcome) |
| Branch | Create git branch | Skip unless task produces code |
| Work | Code + test + commit | Draft, review, publish, or configure |
| Ship | PR + merge | Post/publish/submit + verify live |
| Update | Update BACKLOG.md | Update GTM-BACKLOG.md + weekly-log.md |
| Review | Session retro | Same format, add `gtm` tag |

**GTM "Ship" definitions by type:**
- **Distribution:** post is live, link verified
- **Onboarding:** walkthrough complete, bugs filed
- **Instrumentation:** events firing in production
- **Community:** channel created and linked from README
- **Sales:** asset committed in `docs/gtm/`

**When GTM work requires code** (instrumentation, landing page edits): use full engineering workflow with branch + PR. GTM backlog item stays open until both code ships AND GTM outcome is verified.

## GTM Context

For marketing, sales, and content work:
- **Backlog:** `docs/gtm/GTM-BACKLOG.md` — prioritized GTM tasks (Now/Next/Later with ICE scoring)
- **Context:** `docs/gtm/GTM-CONTEXT.md` — ICP, CTAs, content pillars, and GTM docs index

# currentDate
Today's date is 2026-02-20.
