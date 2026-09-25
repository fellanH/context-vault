> **Archived 2026-07-03.** The actual cutover skipped the staged Stage-1-IPC approach below and
> went straight to deleting the TS engine (2026-07-02). Current status lives in
> `project-management/docs/rust-cutover-runbook.md`. Kept for historical context.

# npm → Rust CLI migration

**Status:** Superseded — see archive note above and `specs/rust-core.md` for the design that shipped.

## Three-stage migration

**Stage 1 — Rust engine behind IPC** (current target)
TypeScript MCP server shells out to `context-vault invoke --stdin-json` for all engine operations. Users see no change. Validates the Rust engine against production traffic.

**Stage 2 — Full Rust MCP server**
`packages/local` becomes a thin wrapper that ships the Rust binary. The `context-vault` npm package uses `optionalDependencies` per-platform (same pattern as `esbuild`, `vite`, `@biomejs/biome`). No Node.js runtime required for the MCP server. Cold start drops from ~300ms to <10ms.

**Stage 3 — WASM engine in Workers**
Same Rust engine compiled to `wasm32-unknown-unknown`, replacing the Kysely vault layer in `context-vault-app/server`. Local and hosted tiers run identical search logic.

## PATH collision note (Stage 1 → 2 transition)

During Stage 1 the TypeScript npm package and the Rust binary share the name `context-vault` on PATH. For MCP configs and automation, set `CONTEXT_VAULT_CLI` to the absolute path of the binary you want invoked. Humans: install Rust build to `~/.local/bin-rust` and only prepend for testing.

## Semver / cutover rule

- Ship Rust binaries at GA quality (Stage 2 complete, ranking parity verified by bench harness).
- Next npm major after GA introduces the download shim or dual-binary behavior.
- No automatic migration until telemetry and docs confirm readiness.

## Migration banner (for package README when Stage 1 ships)

> **Native CLI preview:** A Rust `context-vault` binary is available. The npm package remains supported. If both are on PATH, set `CONTEXT_VAULT_CLI` to the native binary path in your MCP config. See `docs/rust-distribution.md`.
