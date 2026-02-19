# Context Vault Marketing Site

Standalone marketing website for Context Vault.

## Goals

- Keep marketing pages isolated from the product dashboard app (`packages/app`)
- Reuse the same visual language (tokens, typography, component primitives)
- Support a high-conversion landing page and blog routes

## Routes

- `/` - Landing page
- `/blog` - Blog index
- `/blog/:slug` - Blog post detail

## Local Development

From repo root:

```bash
npm install
npm run marketing:dev
```

Build:

```bash
npm run marketing:build
```

## App Link Target

- `VITE_APP_BASE_URL` (optional): if set, all app conversion links (for example `/register`) point to that origin.
- Default is same-origin behavior (links stay relative).
