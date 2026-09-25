# Rust binary distribution

See `specs/rust-core.md` for engine design and `docs/npm-rust-migration.md` for the migration stages. This file covers the binary packaging and release mechanics only.

## Target triples

Pre-built binaries ship for:

| Triple | Platform |
|--------|----------|
| `x86_64-unknown-linux-gnu` | Linux x64 (servers, CI) |
| `aarch64-unknown-linux-gnu` | Linux ARM64 (Hetzner ARM, Raspberry Pi) |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows x64 |

## GitHub Actions release workflow

`.github/workflows/rust-release.yml` (to be created at Stage 2):
1. Matrix build across all target triples
2. `cargo build --release` per triple
3. Aggregate artifacts, emit `SHA256SUMS`
4. Attach binaries + checksums to GitHub Release

## npm shim (Stage 2)

The `context-vault` npm package gains an `optionalDependencies` block per platform (same pattern as `esbuild`). Each platform package contains a pre-built binary. The main package's `bin/context-vault` wrapper prefers the native binary if present, falls back to the TypeScript CLI.

```json
{
  "optionalDependencies": {
    "@context-vault/cli-darwin-arm64": "x.y.z",
    "@context-vault/cli-darwin-x64": "x.y.z",
    "@context-vault/cli-linux-x64": "x.y.z",
    "@context-vault/cli-win32-x64": "x.y.z"
  }
}
```

## PATH collision: npm vs Rust (Stage 1 only)

During Stage 1, TypeScript npm package and Rust binary share the name `context-vault` on PATH.

- **MCP configs / automation:** set `CONTEXT_VAULT_CLI=/path/to/rust/context-vault`
- **Humans:** install Rust build to `~/.local/bin-rust`, only prepend for testing
- **Checksum verification:** verify `SHA256SUMS` before `chmod +x` on downloaded binary

## Local build

```bash
cargo build -p context-vault --release
# binary at: target/release/context-vault

cargo install --path crates/context-vault --locked
# installs to: ~/.cargo/bin/context-vault
```

## FTS bench note

See `docs/bench-vault-fts.md` for `--vault-db` FTS token selection behaviour used in the bench harness.
