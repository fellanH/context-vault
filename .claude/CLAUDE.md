# Context Vault — Project Context

Persistent memory layer for AI agents via MCP. Markdown files + SQLite FTS + semantic embeddings.

| Resource   | URL                                                                                            |
| ---------- | ---------------------------------------------------------------------------------------------- |
| Repo       | `https://github.com/fellanH/context-vault`                                                     |
| Marketing  | `https://contextvault.dev`                                                                     |
| App        | `https://context-vault.com`                                                                    |
| Quickstart | `https://github.com/fellanH/context-vault/blob/main/docs/distribution/connect-in-2-minutes.md` |

## Tone

- Technical, helpful, honest about trade-offs
- No hype, no fabricated metrics — show real commands, real output
- Speak as a developer building tools for other developers

## Session Protocol

Full reference: see `DEV-PLAYBOOK.md` in the project root.

Every Claude Code working session follows this workflow:

### 0. Session Goal (before anything else)

Ask the user: **"What is the one thing this session should accomplish?"**
If they haven't stated it, prompt for it. Hold this goal for the entire session.

If the user makes a request that doesn't serve the declared goal, name the conflict and offer:

1. **Capture it** — add to `INBOX.md` and keep going
2. **Pivot** — explicitly stop current work, move to `Next`, start the new thing
3. **Queue it** — handle after the current task is done

Never silently pivot. The user always chooses.

### Quick Capture (handle at any point in the session)

If the user's message starts with `capture:` — or if they're clearly in idea-generation mode (several short "what about X / we should also Y" prompts in a row) — do NOT implement. Instead:

- Append the idea to `INBOX.md` in the format: `YYYY-MM-DD — [idea]`
- Confirm it's logged in one line
- Return immediately to whatever was in progress

If the user sends 3+ short generative prompts in a row, shift to capture mode and say:
_"Looks like you're in capture mode — I'll log these and pause implementation. Keep going, what else?"_

For explicit brainstorm sessions: `chaos mode on` → everything goes to INBOX.md until `chaos mode off`.

### 1. Orient

- Run `gh pr list` — any open PRs? Must resolve (merge/close/continue) before starting new work
- Read `BACKLOG.md` — check `Now` against session goal; they should match
- Glance at `INBOX.md` — note what accumulated; don't process it here
- Run `git branch` — clean up stale local branches
- Query context vault: `get_context` with tags `context-vault`

### 2. Pick

- One item from `Now` that matches the session goal (or pull highest-ICE from `Next` if `Now` is empty)
- GTM work: pick from `docs/gtm/GTM-BACKLOG.md`
- Hard cap: 3 items in `Now` at any time

### 3. Pitch

Present plan for user approval **before writing code**.

- **Non-trivial**: Why this/why now, 2–3 options with trade-offs, scope & risk
- **Trivial**: One-line summary, proceed after approval
- **Open PR check**: If a PR is already open, explicitly merge it, close it, or confirm continuing it — never start new work on top silently

### 4. Branch

- Create `feat/<scope>-<slug>`, `fix/<scope>-<slug>`, or `chore/<slug>` — no direct commits to `main` (#21)
- Scope = the primary package changed: `marketing | hosted | cli | extension | core | infra`
- One agent, one branch. Never commit to a branch owned by another agent in the same session
- GTM-only tasks with no code changes skip this step
- See branch prefix map in `DEV-PLAYBOOK.md`

### 5. Work

- Implement, test, commit with conventional messages
- Reference issues: `Fixes #N`

### 6. Ship

- Push, wait for CI, then `gh pr merge --squash --admin`
- PR body includes `Fixes #N` to auto-close issues
- Move item from `Now` → `Done` in `BACKLOG.md`

### 7. Update

- Update `BACKLOG.md`: move completed items, add signals, adjust priorities
- File GitHub issues for newly discovered work
- Do not add documentation for anything derivable from code

### 8. Review

End-of-session retrospective: **Shipped**, **Went well**, **Friction**, **Learned**, **Actions**.

- Dump any ideas that surfaced during the session → `INBOX.md`
- Delete merged branches: `git branch -d <branch>` locally + `git push origin --delete <branch>`
- Persist: `save_context` (kind: `session-review`, tags: `context-vault, retro`)
- Durable lessons → memory files or CLAUDE.md. Actionable friction → issues or backlog

### Branch naming

| Prefix   | Use                  |
| -------- | -------------------- |
| `feat/`  | New features         |
| `fix/`   | Bug fixes            |
| `chore/` | Infra, deps, cleanup |

### Sprint Tracking

BACKLOG.md `Now` must mirror open PRs at all times:

| Event                   | Action                   |
| ----------------------- | ------------------------ |
| PR opened               | Move item to `Now`       |
| PR merged               | Move item to `Done`      |
| PR closed without merge | Move item back to `Next` |

Hard cap: 3 items in `Now`. If `Now` has an item with no matching open PR, stop and investigate before proceeding.

### Issue labels

| Label          | Purpose                          |
| -------------- | -------------------------------- |
| `bug`          | Something broken                 |
| `feature`      | New capability                   |
| `enhancement`  | Improvement to existing          |
| `dx`           | Developer experience             |
| `infra`        | CI/CD, deployment                |
| `gtm`          | Marketing, sales, content        |
| `user-request` | Directly from a user             |
| `P0`–`P3`      | Priority (critical → eventually) |

### ICE scoring

Score = Impact(1-5) × Confidence(1-5) × Ease(1-5). Highest → top of `Next`.

### Weekly triage

Review feedback entries + issues, add community signals, re-score `Next`, pull top items into `Now` (max 3).

### GTM Workflow

Same protocol, different sources:

| Step   | Engineering  | GTM                                 |
| ------ | ------------ | ----------------------------------- |
| Orient | `BACKLOG.md` | `GTM-BACKLOG.md` + `GTM-CONTEXT.md` |
| Branch | Always       | Skip unless code changes needed     |
| Ship   | PR + merge   | Post/publish + verify live          |
| Update | `BACKLOG.md` | `GTM-BACKLOG.md` + `weekly-log.md`  |

**GTM "Ship" definitions:** Distribution = post live + link verified. Onboarding = walkthrough complete. Instrumentation = events firing. Community = channel created. Sales = asset committed.

When GTM requires code: full engineering workflow. GTM item stays open until both code ships AND outcome is verified.

## GTM Context

- **Backlog:** `docs/gtm/GTM-BACKLOG.md`
- **Context:** `docs/gtm/GTM-CONTEXT.md` — ICP, CTAs, content pillars
