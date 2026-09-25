# Development Workflow

How to make changes to context-vault, release them, and verify they work as a real user would experience.

## Prerequisites

- npm account with publish access (automation token in omni secrets: `npm secrets get npm authToken`)
- GitHub CLI (`gh`) authenticated with push access to `fellanH/context-vault`
- Node.js 22+ required
- Rust toolchain (`rustup`) for `crates/context-vault` work

## Repo structure

```
context-vault/
├── packages/
│   ├── core/          # @context-vault/core (shared library, no CLI deps)
│   ├── local/         # context-vault (CLI + MCP server, depends on core)
│   ├── sdk/           # @context-vault/sdk (hosted + local client)
│   └── hosted-gateway/ # self-hosted HTTP wrapper (not SaaS)
├── crates/
│   └── context-vault/ # Rust engine (bench harness today; real engine in progress)
├── scripts/
│   └── release.mjs    # Automated release script
├── test/              # Unit, integration, and E2E tests
├── specs/             # Architecture + design specs (read before starting work)
└── CHANGELOG.md       # Must be updated before every release
```

`local` depends on `core` via npm workspace. After modifying core, rebuild it before local, or run `npm run build` from the monorepo root (builds core first, then local).

## TypeScript development cycle

### 1. Make changes

Edit source in `packages/core/src/` and/or `packages/local/src/`.

### 2. Build

```bash
npm run build     # builds core → local in order
```

### 3. Typecheck

```bash
npx tsc --noEmit 2>&1 | head -20
```

If `local` can't see new types from `core`, the workspace link may be stale. Run `npm install` from the monorepo root to re-resolve, then rebuild.

### 4. Test

```bash
npm test          # runs vitest across all packages
```

All tests must pass before release. The release script runs them again as a gate.

### 5. Update CHANGELOG.md

Add a section for the new version **before** running the release script.

```markdown
## [x.y.z] — YYYY-MM-DD

### Fixed / Added / Changed
- Description of change
```

### 6. Release

```bash
node scripts/release.mjs patch   # x.y.z → x.y.(z+1)
node scripts/release.mjs minor   # x.y.z → x.(y+1).0
node scripts/release.mjs major   # x.y.z → (x+1).0.0
```

The script: bumps versions (root + core + local) → runs tests → builds → publishes `@context-vault/core` then `context-vault` → commits, tags, pushes → creates GitHub Release.

**Requires clean working tree.** Commit or stash all changes first.

## Rust development cycle

See `specs/rust-core.md` for the full engine spec and migration stages.

```bash
# Build
cargo build -p context-vault

# Run bench (against live vault)
cargo run -p context-vault -- bench --vault-db ~/.context-mcp/vault.db

# Run bench (fixture corpus)
cargo run -p context-vault -- bench --format json

# Install locally
cargo install --path crates/context-vault --locked
```

The Rust crate must maintain ranking parity with the TypeScript core — use `cargo run -- bench --vault-db` to verify before shipping changes.

## Post-release verification

After publishing, verify the release works as a real user would experience it.

```bash
# Install as user
npm install -g context-vault@latest
context-vault --version   # verify version matches release
context-vault setup       # configure MCP tools
```

In a Claude Code session:
1. `context_status` — should show vault info and current version
2. `save_context` with a test entry
3. `get_context` with a matching query

## Troubleshooting

### "skipEmbeddings" type error on build

Core was rebuilt but local still sees the old `.d.ts`. Run `npm install` from monorepo root to re-link, then `npm run build`.

### Release script says "Working tree is dirty"

Untracked files count as dirty. Commit them, add to `.gitignore`, or stash.

### npx serves a stale version

```bash
npx context-vault@latest --version
# or clear the cache:
rm -rf ~/.npm/_npx/
```
