# Development Strategy Playbook

**Design philosophy:** This system is built for fast, generative work. It assumes you will have ideas mid-session, will sometimes lose track of the bigger picture, and will want to run things in parallel. The system's job is to protect you and the product from those tendencies — not slow you down.

**Update this when the workflow changes — not on a schedule.**

---

## The Three-Layer System

```
┌─────────────────────────────────────────────────────────────┐
│  CAPTURE LAYER — INBOX.md                                   │
│  Every idea lands here. Nothing is acted on in the moment.  │
│  Quick capture: "capture: [idea]" mid-session               │
└───────────────────────┬─────────────────────────────────────┘
                        │ weekly triage
                        ▼
┌─────────────────────────────────────────────────────────────┐
│  PLANNING LAYER — BACKLOG.md                                │
│  ICE-scored, sequenced. Now = open PRs (max 3).            │
│  Nothing moves to execution without being scored first.     │
└───────────────────────┬─────────────────────────────────────┘
                        │ session pick
                        ▼
┌─────────────────────────────────────────────────────────────┐
│  EXECUTION LAYER — Active Session                           │
│  One declared goal. Agents scoped and branched.            │
│  Claude guards this layer from interruption.               │
└─────────────────────────────────────────────────────────────┘
```

**The rule:** ideas flow down, never sideways. A new idea mid-execution goes UP to capture, not sideways into the current session.

---

## Part 1 — Quick Capture (the most important habit)

When an idea hits mid-session — a feature, a bug you noticed, a scope change, a "we should also..." — **do not act on it yet.** Capture it and keep going.

**The capture command:**

```
capture: [your idea in any form — rough is fine]
```

Claude will add it to `INBOX.md` with today's date and return to whatever was in progress. Nothing stops. Nothing pivots. The idea is safe.

**Examples:**

- `capture: add dark mode to the app dashboard`
- `capture: the CLI --help output is confusing, redesign it`
- `capture: should we support webhook notifications for vault events?`
- `capture: blog post idea — "why we chose local-first over cloud-first"`

**The inbox is a judgment-free zone.** Don't filter before capturing. Rough ideas, half-baked thoughts, "probably won't do this but..." — all of it belongs in the inbox. Judgment happens at triage, not capture.

---

## Part 2 — Starting a Session

### Step 1: Declare your session goal

Before anything else, state the one thing this session should accomplish:

```
Session goal: [one sentence]
```

Examples:

- _"Session goal: ship the rate limiting feature on the hosted API to main."_
- _"Session goal: draft Campaign B assets for X and Reddit."_
- _"Session goal: triage the inbox and reorder the backlog."_

If you can't state it in one sentence, the session is too broad. Split it.

Claude holds this goal for the session. If you make a request that doesn't serve it, Claude will flag it and offer to capture instead.

### Step 2: Orient

```bash
gh pr list                    # open PRs = active work, must resolve before starting new
cat BACKLOG.md                # what's in Now? is it consistent with your session goal?
cat INBOX.md                  # what accumulated since last session? (don't process now — just aware)
git branch                    # any stale local branches to clean up?
```

**What to do with what you find:**

| State                             | Action                                                       |
| --------------------------------- | ------------------------------------------------------------ |
| Open PRs match session goal       | Continue them                                                |
| Open PRs don't match session goal | Merge, close, or consciously defer — then proceed            |
| `Now` is empty                    | Pull the highest-ICE item from `Next` that matches your goal |
| `Inbox` has items                 | Note them, don't process — that's for triage sessions        |
| Stale branches                    | `git branch -d <name>` before starting                       |

### Step 3: Pick and go

One item from Now (or the top of Next if Now is empty). That's the session. If you want to do more than one thing, check Part 5 for whether it's parallel-safe.

---

## Part 3 — The Guardian System

Claude's job is to protect your session goal and your in-flight work from your own erratic prompts. Here's how it works:

### When you go off-script mid-session

If you make a request that would derail in-flight work or conflict with your session goal, Claude will:

1. **Name the conflict** — "Agent X is mid-implementation on branch Y. This request would touch the same files / abandon that work."
2. **Offer three options:**
   - **Capture it** — `capture:` it and keep going
   - **Pivot** — explicitly stop current work, move it to Next, start the new thing
   - **Queue it** — add it to the end of this session (after current work is done)

You always choose. Claude never silently pivots.

### When you lose the thread

If several prompts go by without a clear connection to the session goal, Claude will surface it:

_"Quick check — session goal was [X]. Current request looks like [Y]. Are we still on goal, or do you want to update the goal?"_

This is not a blocker. It's a nudge. Say "yes, on goal" and Claude keeps going. Say "no, new goal is Z" and the session goal updates.

### When you're in idea-generation mode

If you send several short prompts quickly — "what about X", "we should also Y", "can we add Z" — Claude will recognize the pattern and shift to capture mode:

_"Looks like you're in capture mode. I'll log these to the inbox and pause implementation until you're ready to pick one. Keep going — what else?"_

This prevents you from accidentally starting 4 things at once.

### Chaos mode (explicit)

If you want to brainstorm without triggering any implementation, say:

```
chaos mode: [everything that follows is capture only, nothing gets built]
```

Claude will capture everything to INBOX.md with zero implementation until you say:

```
chaos mode off
```

---

## Part 4 — Picking Your Path

Answer three questions:

```
1. Does this touch code?
   No  → Path E (GTM/Content) — no branch, no CI
   Yes → continue

2. How many deliverables?
   One                        → Path A (single engineering)
   Several, no file overlap   → Path B (parallel agents)
   Several, shared files      → sequential Path A sessions

3. Is main clean, nothing open?
   Yes + you want to ship → Path C (release)
   No                    → finish what's open first
```

If you're unsure what to build at all → **triage session** (see Part 7).

---

## Part 5 — Prompt Templates

### Starting a feature or fix

```
Session goal: [outcome in one sentence]

I want to [what] in [packages/X].
Orient first, pitch approach before writing code.
```

### Quick fix (you know exactly what it is)

```
Fix [specific thing] in [file or package].
Branch, implement, PR. No pitch needed.
```

### Continuing an open PR

```
Continue work on PR #[N] / branch [name].
Read the diff to understand what's done. Remaining work: [describe].
```

### Parallel sprint (Path B)

```
Session goal: [outcome for all tasks]

I have [N] independent tasks — confirm zero file overlap before spawning:
1. [Task A] — touches [packages/X] only
2. [Task B] — touches [docs/gtm/Y] only
3. [Task C] — touches [packages/Z] only

Assign branches, spawn in parallel, each opens its own PR.
```

### Planning before building

```
I want to think through [problem] before writing any code.
Read [relevant files]. Give me 2–3 approaches with trade-offs.
No branch, no implementation — just the plan.
```

### Release

```
Run a release. Confirm Now is empty, no open PRs, status is clean.
Release type: [patch / minor / major].
```

### Triage session

```
Triage session. No code.
Read INBOX.md, BACKLOG.md, and the last 5 commits.
1. Process inbox items: ICE score each, move to Next/Later/discard.
2. Reorder Next by ICE.
3. Confirm Now matches open PRs.
4. Tell me what to pull into Now for the next session.
```

Run this weekly or whenever you feel lost.

### Strategic re-anchor (use when you've lost the thread)

```
I've lost track. No code.
Read BACKLOG.md, gh pr list, INBOX.md, and DEV-PLAYBOOK.md Part 7 (strategy).
Tell me: what's in flight, what should be next, and whether what I've been
building is serving the 90-day goal.
```

---

## Part 6 — When to Plan vs Jump In

**Plan first when:**

- Change touches more than 2 packages
- You don't know where the code lives
- Multiple valid approaches exist with real trade-offs
- Schema change, new dependency, or new API contract involved
- You haven't worked in that area recently

**Jump straight in when:**

- File and fix are both obvious
- Single-package change, clear scope
- Pattern you've done before
- Doc-only or config change

### The planning conversation

1. **Describe the goal, not the solution.** _"I want users to export their vault as JSON"_ — not _"I want a /export endpoint."_ The goal framing lets Claude find the right abstraction.

2. **Ask for options, not answers.** _"Give me 2–3 approaches with trade-offs"_ — not _"How do I do this?"_

3. **Scope the risk.** _"What's the blast radius? What breaks if this goes wrong? Is it reversible?"_ Mandatory for anything touching the hosted API or DB schema.

4. **Lock scope explicitly before approving.** _"We're doing X only, not Y. Don't refactor the surrounding code."_ Agents expand scope when it's not bounded.

---

## Part 7 — Parallelism

**The rule: parallel = zero shared files.** One shared file = sequential, no exceptions.

### Parallel-safe combinations

| Task A                                     | Task B                                    | Why                             |
| ------------------------------------------ | ----------------------------------------- | ------------------------------- |
| Marketing page (`packages/marketing`)      | Blog post (`docs/gtm/assets`, `posts.ts`) | Different packages              |
| Hosted API fix (`packages/hosted`)         | CLI improvement (`packages/local`)        | Separate packages               |
| Two campaign assets for different channels |                                           | Separate files                  |
| Two unrelated test files                   |                                           | No shared imports being changed |

### Not parallel-safe

| Combination                      | Why it breaks                              |
| -------------------------------- | ------------------------------------------ |
| Feature + tests for that feature | Tests import the new code — race condition |
| Two changes to `packages/core`   | Core is shared — ripple effects            |
| Any feature work + release       | Release needs clean state                  |
| Two agents on same package       | Same component files, overwrites           |
| Two BACKLOG.md updates           | Same file, merge conflict                  |

### The foundation-first pattern

When tasks look parallel but share a dependency:

1. Identify the shared foundation
2. Build it first as Path A (single agent)
3. Parallelize the consumers once foundation is merged

_Example: core change + app UI + CLI → core first, then app + CLI in parallel._

### How many agents

| Count | When justified                                         |
| ----- | ------------------------------------------------------ |
| 1     | Always the default — zero overhead                     |
| 2     | Clearly separate packages, clear success criteria each |
| 3     | All truly independent AND scope is locked              |
| 4+    | Rarely justified — review overhead exceeds the savings |

---

## Part 8 — Strategy: Getting More From Your Effort

### The three places effort goes

1. **Picking** — choosing what to build
2. **Framing** — setting up agents to succeed
3. **Reviewing** — closing the loop

Wasted effort almost always lives in picking (wrong priority) or framing (too much latitude). Reviewing is fast when framing was good.

### Picking: ICE, not recency or urgency

BACKLOG.md `Next` is scored by ICE (Impact × Confidence × Ease). Highest score goes first.

Before every session: _"Is this the highest-ICE item available? If not, do I know why I'm skipping it?"_

**The traps:**

- **Urgency bias** — doing what feels pressing over what's high-value
- **Novelty bias** — new features over existing bugs with 10× the user impact
- **Completion bias** — polishing good-enough instead of moving to what's next
- **Recency bias** — building the last thing you thought of instead of the best thing

### Framing: what agents need to succeed

**Works:**

- Explicit file scope (_"only touch packages/marketing/"_)
- Clear success criterion (_"build passes, new section renders at /pricing"_)
- Named branch (_"branch: feat/marketing-pricing"_)
- Context on WHY (_"users ask about pricing before they ever see the app"_)

**Fails:**

- Ambiguous verbs (_"improve", "refactor", "clean up"_)
- Scope crossing multiple packages without a seam
- Decisions requiring business context the agent doesn't have
- _"See what you can find"_

If your prompt has ambiguous verbs or undefined scope, add a planning step. Ask for a plan before asking for code.

### Reviewing: read the diff

When an agent opens a PR, read the diff — not just the description. The description is what the agent intended. The diff is what happened. Check for scope creep (files outside the assignment). For anything user-facing or touching the hosted API, run it locally.

Good review takes 2–5 minutes when the agent was well-framed. If it takes longer, it's a framing problem.

### One deliverable per session

The single most reliable pattern: finish one thing completely before starting the next.

- One PR open (usually)
- Session ends with PR merged, BACKLOG.md updated
- No "I'll come back to this" branches

### The 90-day anchor

Every individual session should serve the 90-day plan. See `docs/90-DAY-PLAN.md`.

When something you're building doesn't clearly map to the 90-day plan, stop and ask: _"Is this serving the plan, or am I just building what's interesting right now?"_ If you can't answer, run the strategic re-anchor prompt (Part 5).

### Weekly triage ritual (15 minutes)

Run the triage session prompt (Part 5) once a week:

1. Process INBOX.md — ICE score each item, move to Next/Later/discard, clear inbox
2. Re-score Next — did anything change priority since last week?
3. Confirm Now matches open PRs
4. Pick what goes into Now for the coming week (max 3)

This is the pressure valve. The inbox only works if it gets processed. A triage session once a week is enough.

---

## Reference — Session Types

| Type              | When                               | Branch                                        | Who                             |
| ----------------- | ---------------------------------- | --------------------------------------------- | ------------------------------- |
| **Engineering**   | Code feature or fix                | `feat/<scope>-<slug>` or `fix/<scope>-<slug>` | Claude CLI                      |
| **Multi-agent**   | 2–3 independent, zero file overlap | One per agent, assigned upfront               | Team spawn                      |
| **Release**       | After features merged, clean state | None (runs on main)                           | cv-release-manager              |
| **GTM / Content** | Docs, campaigns, blog posts        | None or `feat/gtm-<slug>`                     | cv-campaigns, cv-content-writer |
| **Triage**        | Weekly or when lost                | None                                          | Claude CLI, no code             |
| **Maintenance**   | Cleanup, deps, infra               | `chore/<slug>`                                | Claude CLI                      |

---

## Reference — Branch Ownership

1. One agent, one branch.
2. Branch names encode scope: `feat/<scope>-<slug>`.
3. No direct commits to main. Ever.
4. Branches die at merge — delete local + remote in session review.
5. Check before creating: `git branch` + `gh pr list`.

| Agent              | Allowed prefix                               |
| ------------------ | -------------------------------------------- |
| cv-site-dev        | `feat/marketing-*`                           |
| cv-release-manager | none — clean main only                       |
| cv-test-runner     | none — read-only                             |
| cv-campaigns       | `feat/gtm-*` only if code needed             |
| cv-content-writer  | `feat/content-*` or `feat/gtm-*`             |
| cv-sales-ops       | `chore/gtm-*` only                           |
| Claude CLI         | `feat/<scope>-*`, `fix/<scope>-*`, `chore/*` |

---

## Reference — Sprint Tracking

**BACKLOG.md Now = open PRs. Keep in sync.**

| Event                   | Action                   |
| ----------------------- | ------------------------ |
| PR opened               | Move item to `Now`       |
| PR merged               | Move item to `Done`      |
| PR closed without merge | Move item back to `Next` |

Hard cap: 3 items in `Now`. Now item with no open PR = investigate before proceeding.

---

## Reference — CI/CD Pipeline

```
PR to main
  └── ci.yml: test (always) + conditional builds per changed package

Merge to main
  └── ci.yml → deploy.yml (on CI success)
        ├── backend  — only if packages/hosted/** or packages/core/** changed
        └── frontend — only if packages/app/** or packages/marketing/** changed
              └── staging → health check → smoke → production (manual approval)

Tag push v*
  ├── publish.yml           → npm publish + GitHub Release
  └── publish-extension.yml → Chrome Web Store (manual until CWS secrets set)
```

| Package              | Deploys to                   | Trigger                                 |
| -------------------- | ---------------------------- | --------------------------------------- |
| `packages/local`     | npm                          | tag `v*`                                |
| `packages/hosted`    | Fly.io                       | main push (hosted or core changed)      |
| `packages/app`       | Vercel app.context-vault.com | main push (app or core changed)         |
| `packages/marketing` | Vercel contextvault.dev      | main push (marketing or core changed)   |
| `packages/extension` | Chrome Web Store             | tag `v*` — manual until CWS secrets set |

---

## Reference — Agent Assignments

| Task                             | Agent              | Path           |
| -------------------------------- | ------------------ | -------------- |
| Marketing page, UI, CRO          | cv-site-dev        | A or B         |
| Blog post content                | cv-content-writer  | E              |
| Campaign assets (X, Reddit, HN)  | cv-campaigns       | E              |
| Release                          | cv-release-manager | C              |
| Test analysis                    | cv-test-runner     | support in A/B |
| Feature/fix in hosted, core, CLI | Claude CLI direct  | A              |
| GitHub comment response          | @claude async      | D              |

---

## Reference — Common Failure Modes

| Symptom                        | Cause                         | Fix                                   |
| ------------------------------ | ----------------------------- | ------------------------------------- |
| Agents conflict on push        | Shared branch                 | Assign branches before spawning       |
| Staging overwrites prod Vercel | `--prod` on staging step      | Fixed in deploy.yml                   |
| Every merge deploys everything | No path scoping               | Fixed with dorny/paths-filter         |
| BACKLOG.md Now drifts          | Manual sync skipped           | Sprint tracking convention            |
| Inbox becomes a graveyard      | No weekly triage              | 15-min triage session, weekly         |
| Session derails mid-way        | No declared goal              | Session goal before orient            |
| Stale branches accumulate      | No cleanup                    | Delete at session review              |
| Release version mismatch       | Extension manifest not bumped | Release script handles — double-check |
| npm install fails post-publish | Registry lag (~60s)           | `npm cache clean --force` then retry  |
