#!/usr/bin/env node
// Reference thin launcher for the Rust cutover (Phase 7).
//
// Once the per-platform Rust binaries ship as optionalDependencies
// (@context-vault/cli-<target>, esbuild/@biomejs/biome pattern), `bin/cli.js`
// delegates all ENGINE subcommands (mcp/reindex/status/doctor/search/save/…) to the
// native binary resolved here. The Node launcher keeps only the installer, MCP-client
// registration, and remote/team/bridge glue — none of which touch the vault engine.
//
// This file is the mechanism reference; wiring it into cli.js is the cutover step in
// project-management/docs/rust-cutover-runbook.md.

import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';

const require = createRequire(import.meta.url);

/** Map Node's platform/arch to the Rust target triple used for the package name. */
function targetTriple() {
  const platform = process.platform;
  const arch = process.arch;
  const map = {
    'darwin-arm64': 'aarch64-apple-darwin',
    'darwin-x64': 'x86_64-apple-darwin',
    'linux-arm64': 'aarch64-unknown-linux-gnu',
    'linux-x64': 'x86_64-unknown-linux-gnu',
    'win32-x64': 'x86_64-pc-windows-msvc',
  };
  return map[`${platform}-${arch}`];
}

/** Resolve the native `context-vault` binary from the platform optionalDependency,
 * falling back to $CONTEXT_VAULT_BIN or a locally built target/release binary. */
export function resolveRustBinary() {
  if (process.env.CONTEXT_VAULT_BIN) return process.env.CONTEXT_VAULT_BIN;

  const binName = process.platform === 'win32' ? 'context-vault.exe' : 'context-vault';
  const triple = targetTriple();
  if (triple) {
    const pkg = `@context-vault/cli-${triple}`;
    try {
      const pkgRoot = require.resolve(`${pkg}/package.json`);
      const candidate = join(pkgRoot, '..', 'bin', binName);
      if (existsSync(candidate)) return candidate;
    } catch {
      /* platform package not installed — fall through */
    }
  }

  // Stable local-install fallback: a binary placed at ~/.context-mcp/bin (this
  // product's own data directory), independent of the caller's cwd -- unlike the
  // dev fallback below, this resolves correctly no matter where a hook or shell
  // invokes `context-vault` from. Populated by `scripts/install-local-bin.sh` or
  // manually until the real per-platform npm packages ship.
  const localInstall = join(homedir(), '.context-mcp', 'bin', binName);
  if (existsSync(localInstall)) return localInstall;

  // Dev fallback: a locally built binary, only useful when cwd is this repo.
  for (const rel of ['target/release/context-vault', 'target/debug/context-vault']) {
    const p = join(process.cwd(), rel);
    if (existsSync(p)) return p;
  }
  return null;
}

/** Exec the native engine with the given args; returns the child exit code. */
export function runRustEngine(args) {
  const bin = resolveRustBinary();
  if (!bin) {
    process.stderr.write(
      'context-vault: native engine binary not found. Reinstall `context-vault` so the ' +
        'platform package installs, or set CONTEXT_VAULT_BIN to a built binary.\n'
    );
    return 1;
  }
  const res = spawnSync(bin, args, { stdio: 'inherit' });
  return res.status ?? 1;
}
