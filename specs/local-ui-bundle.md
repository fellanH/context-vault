# Spec: Local Mode — Cloud UI connecting to local instances

**Status:** Planned — not yet implemented
**Owner:** unassigned
**Repos affected:** `context-vault-app` (primary), `context-vault` (minor cli.js cleanup)

---

## Clarification of terms

| Term       | Meaning                                                                                          |
| ---------- | ------------------------------------------------------------------------------------------------ |
| **Local**  | Local markdown vault (`~/vault/`), local SQLite DB (`~/.context-mcp/`), local MCP server (stdio) |
| **Hosted** | Cloud vault on Fly.io, per-user SQLite, MCP over Streamable HTTP                                 |
| **UI**     | Always `app.context-vault.com` — there is no bundled or offline UI                               |

The cloud UI is the only UI. It connects to either a local backend or the hosted backend
depending on mode. Bundling the React app into the npm package is **not needed and not planned**.

---

## Problem

The cloud UI (`app.context-vault.com`) currently only talks to the hosted backend
(`api.context-vault.com`). Users running `context-vault` locally have a fully working
local REST API (`local-server.js`, port 3141) but no way to point the UI at it.

`context-vault ui` already attempts to open a browser with `?local=<port>` but the React
app does not read or act on that parameter — it always hits the hosted API.

---

## Goal

The cloud UI should support two modes, switchable at runtime:

| Mode       | API base                        | Auth         | Data                      |
| ---------- | ------------------------------- | ------------ | ------------------------- |
| **Local**  | `http://localhost:{port}`       | none         | local SQLite + `~/vault/` |
| **Hosted** | `https://api.context-vault.com` | Bearer token | Fly.io per-user DB        |

Users can switch modes from within the UI without a page reload. Mode persists across
page reloads via `localStorage`.

---

## Architecture

```
Browser → app.context-vault.com (Vercel, always)
              ↓ API calls go to one of:
┌─────────────────────────┐    ┌──────────────────────────────┐
│  localhost:3141          │    │  api.context-vault.com       │
│  local-server.js         │    │  Hono + Fly.io               │
│  local SQLite + ~/vault/ │    │  per-user cloud DB           │
└─────────────────────────┘    └──────────────────────────────┘
```

`local-server.js` already implements the full REST API surface the UI needs:
`/api/vault/*`, `/api/vault/search`, `/api/vault/status`, `/api/me`, `/api/health`,
`/api/local/link`, `/api/local/sync`, `/api/local/connect`, `/api/local/browse`.
CORS for `localhost:*` origins is already allowed. No changes needed to `local-server.js`.

---

## What needs to be built

### 1. Mode state — single source of truth (`context-vault-app`)

Create a `mode` store (or extend `AuthProvider`) with:

```typescript
type VaultMode = "local" | "hosted";

interface ModeState {
  mode: VaultMode;
  localPort: number; // default 3141
  setMode: (mode: VaultMode, port?: number) => void;
}
```

**Persistence:** `localStorage` keys `cv_mode` and `cv_local_port`.

**Initialization order:**

1. Check `?mode=local&port=3141` query param (set by CLI when opening browser) — if present,
   store to localStorage and strip from URL (`history.replaceState`).
2. Fall back to `localStorage` value.
3. Default to `'hosted'`.

### 2. API client (`src/app/lib/api.ts`)

`getBaseUrl()` must return the correct base depending on mode:

```typescript
function getBaseUrl(): string {
  if (getMode() === "local") {
    return `http://localhost:${getLocalPort()}`;
  }
  return "https://api.context-vault.com";
}
```

In local mode, omit the `Authorization` header entirely — `local-server.js` has no auth.

### 3. Auth guard (`src/app/components/AuthGuard.tsx`)

In local mode, bypass the auth check and login redirect entirely. The local server's
`/api/me` returns a synthetic `{ userId: 'local', email: 'local@localhost', tier: 'free' }`
— use this to populate the user context so the rest of the app renders normally.

### 4. `VaultModePopover` — wire to real mode state

The component already exists. Connect it to the mode store so selecting local/hosted
actually calls `setMode()`. When switching to local:

- Verify the local server is reachable (`GET /api/health` → `{ ok: true, mode: 'local' }`)
- If unreachable, show an error: "Local server not running. Run `context-vault ui`."
- If reachable, switch mode and refetch all data.

When switching to hosted:

- If no API key is stored, redirect to login/connect flow.
- Otherwise switch and refetch.

### 5. Settings pages in local mode

Hide or replace with upgrade prompts: Account, ApiKeys, Billing.
Show instead: vault path, DB size, link-to-hosted option.
The `/api/vault/status` and `/api/me` endpoints already return the data needed for this.

### 6. `context-vault` CLI — `ui` command cleanup (`bin/cli.js`)

The existing logic (lines ~1038–1112) already:

- Starts `local-server.js` on the given port
- Opens a browser to `https://context-vault.com?local=<port>`

Changes needed:

- Update URL to `https://app.context-vault.com?mode=local&port=<port>` (standardise param names to match what the app reads)
- Remove the dead `bundledDist` / `workspaceDist` check (already cleaned up in prepack.js — verify `cli.js` is also clean)
- Verify `local-server.js` starts correctly and `/api/health` responds before opening browser

---

## Files to create / modify

### `context-vault-app/`

| File                                                         | Change                                                                                           |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `src/app/lib/api.ts`                                         | `getBaseUrl()` reads mode from store; omit auth header in local mode                             |
| `src/app/lib/auth.ts` or new `src/app/lib/mode.ts`           | Mode store: `getMode()`, `setMode()`, `getLocalPort()`, localStorage persistence, URL param init |
| `src/app/components/AuthProvider.tsx`                        | Expose `mode` in context; skip auth flow in local mode                                           |
| `src/app/components/AuthGuard.tsx`                           | Passthrough when `mode === 'local'`                                                              |
| `src/app/components/VaultModePopover.tsx`                    | Wire to mode store; add health check on switch to local                                          |
| Settings pages (`Account.tsx`, `ApiKeys.tsx`, `Billing.tsx`) | Conditionally hide or replace content in local mode                                              |

### `context-vault/`

| File                        | Change                                                                                                      |
| --------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `packages/local/bin/cli.js` | Update `ui` command URL from `context-vault.com?local=PORT` to `app.context-vault.com?mode=local&port=PORT` |

---

## What is explicitly out of scope

- Bundling the React app into the npm package — not needed.
- `app-dist/` directory in the npm package — not needed.
- `scripts/build-ui.mjs` — not needed.
- Offline / air-gapped UI support — the cloud UI is always required.
- Running MCP via the web UI — MCP remains stdio-only for local mode.

---

## Acceptance criteria

- [ ] `context-vault ui` opens `app.context-vault.com?mode=local&port=3141` in the browser
      and starts `local-server.js`.
- [ ] The app boots in local mode, no auth prompt, reads vault entries from local SQLite.
- [ ] Create/edit/delete/search all work against the local DB.
- [ ] `VaultModePopover` switches mode at runtime; data refetches immediately.
- [ ] Switching to local when local server is not running shows a clear error.
- [ ] Mode and port persist in `localStorage`; refreshing the page stays in the same mode.
- [ ] Hosted mode still works exactly as before — no regression.
- [ ] Settings pages adapt to local mode (hide billing/keys, show vault path/status).

---

## Implementation order

1. **`context-vault-app` first:** Ship local mode support to `app.context-vault.com`
   (auth bypass, API base URL, mode store, VaultModePopover wired).
2. **`context-vault` second:** Update the URL in `cli.js` `ui` command to match the new
   param names. This is a one-liner.

Step 1 can be verified manually by navigating to `app.context-vault.com?mode=local&port=3141`
with `local-server.js` running, before touching the CLI at all.
