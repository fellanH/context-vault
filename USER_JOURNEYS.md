# User Journeys — Context Vault

Five core journeys that define how users discover, adopt, and grow with Context Vault.

---

## 1. Local-First Developer

The purist path. No account needed. Vault is markdown files on disk. Everything stays local.

**Flow:**

1. Discovers Context Vault (landing page, GitHub, blog, word of mouth)
2. `npm install -g context-vault`
3. `context-mcp setup` — detects installed AI tools (Claude Code, Cursor, Codex, Windsurf, etc.), configures MCP, downloads embedding model
4. Uses AI tool normally — MCP tools are available automatically
5. Tells AI: "Save an insight: React hooks should..." → entry saved as markdown in `~/vault/`
6. Later asks AI: "What do I know about React hooks?" → hybrid search returns the entry
7. Optionally runs `context-mcp ui` → local dashboard at `localhost:3141` for browsing, searching, managing entries

**Entry points:** Landing page "See 2-minute setup", GitHub README, npm
**Auth:** None
**Value prop:** Persistent AI memory with zero cloud dependency

---

## 2. Cloud-First User

Fastest to value. No local vault setup — everything hosted.

**Flow:**

1. Lands on `www.context-vault.com`
2. Clicks "Start free" → redirected to `/register`
3. Signs up via Google OAuth → account created, API key generated
4. Registration confirmation shows API key + connect command
5. Copies `npx context-vault connect --key cv_...` → runs in terminal
6. Connect command auto-detects AI tools and configures MCP to point at hosted vault
7. Uses AI tools — context saved to and searched from the cloud
8. Logs into `app.context-vault.com` → dashboard shows recent entries, usage meters, onboarding progress
9. Installs Chrome extension (optional) → searches vault from any AI chat UI

**Entry points:** Landing page "Start free" CTA, direct link to `/register`
**Auth:** Google OAuth (primary), email registration (fallback)
**Value prop:** Works across devices, no local setup beyond the connect command

---

## 3. Local → Cloud Migration

The upgrade path for existing local users who want sync, multi-device, or team features.

**Flow:**

1. User already has a local vault with entries (Journey 1)
2. Registers on `app.context-vault.com` → gets API key
3. `context-mcp migrate --to-hosted --key cv_...` → uploads local entries to cloud
4. Local vault files remain untouched (safe backup)
5. `context-mcp connect --key cv_...` → switches AI tools to hosted MCP endpoint
6. Dashboard shows "Upload your local vault?" prompt if entries detected
7. New entries go to cloud; local vault preserved as archive

**Entry points:** Dashboard upload prompt, CLI `migrate` command
**Auth:** Requires hosted account + API key
**Value prop:** Keep everything from local, gain cloud benefits (multi-device, backup, dashboard)

---

## 4. Chrome Extension User

Cross-platform context injection — search your vault from any AI chat interface.

**Flow:**

1. Installs Context Vault extension from Chrome Web Store
2. Opens extension popup → "Connect Your Vault" card
3. Goes to Settings → enters hosted URL + API key → saves
4. Extension shows green status indicator (connected)
5. Opens ChatGPT / Claude.ai / Gemini → extension content script activates
6. Clicks extension → searches vault → selects a result
7. "Inject" button inserts the context snippet into the active chat input
8. Extension auto-closes after injection

**Entry points:** Chrome Web Store, dashboard link, documentation
**Auth:** API key (configured in extension settings)
**Value prop:** Use your vault knowledge in any AI chat, not just MCP-connected tools

---

## 5. Pro Upgrade

Revenue conversion triggered by usage limits.

**Flow:**

1. Free cloud user accumulates entries / makes frequent requests
2. Dashboard usage meters show limits approaching (500 entries, 10 MB, 200 req/day)
3. Rate limit warnings appear (extension banner, API 429 responses)
4. User navigates to `/settings/billing`
5. Sees plan comparison: Free ($0) → Pro ($9/mo) → Team ($29/mo)
6. Clicks "Upgrade to Pro" → Stripe checkout
7. Returns to billing page → tier updated, limits expanded
8. Pro unlocks: 10k entries, 1 GB storage, unlimited requests, 5 API keys, import/export

**Entry points:** Dashboard usage meters, billing settings, rate limit warnings
**Auth:** Already authenticated
**Value prop:** Remove limits, unlock power features

---

## Tier Summary

| | Free | Pro ($9/mo) | Team ($29/mo) |
|---|---|---|---|
| Entries | 500 | 10,000 | Unlimited |
| Storage | 10 MB | 1 GB | 10 GB |
| Requests/day | 200 | Unlimited | Unlimited |
| API Keys | 1 | 5 | Unlimited |
| Support | Community | Priority | Dedicated |

---

## Product Surfaces

| Surface | Purpose | Auth |
|---|---|---|
| Landing page (`www.context-vault.com`) | Discovery, education, CTAs | None |
| Dashboard (`app.context-vault.com`) | Onboarding, browse, search, settings | Google OAuth / API key |
| CLI (`context-mcp`) | Setup, serve, migrate, manage | None (local) or API key (hosted) |
| Chrome Extension | Search + inject context into AI chats | API key |
| MCP Endpoint (`/mcp`) | AI tool integration (save/search/list) | Optional Bearer token |
| REST API (`/api/vault/*`) | Programmatic CRUD + search | Bearer token |

---

## Journey Connections

```
                    ┌──────────────┐
                    │ Landing Page │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              ▼                         ▼
     ┌────────────────┐       ┌─────────────────┐
     │ 1. Local-First │       │ 2. Cloud-First  │
     │   Developer    │       │     User        │
     └───────┬────────┘       └────────┬────────┘
             │                         │
             │    ┌────────────┐       │
             └───►│ 3. Local → │◄──────┘
                  │   Cloud    │
                  └─────┬──────┘
                        │
           ┌────────────┼────────────┐
           ▼                         ▼
  ┌─────────────────┐      ┌─────────────────┐
  │ 4. Extension    │      │ 5. Pro Upgrade  │
  │    User         │      │                 │
  └─────────────────┘      └─────────────────┘
```
