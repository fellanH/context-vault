#!/usr/bin/env node
// Complete a private-source release after CI, without changing source remotes.
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import { validateReleaseTarget } from './release-preflight.mjs';
import { verifyReleaseAssets } from './release-assets.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const run = (bin, args) => execFileSync(bin, args, { cwd: root, encoding: 'utf8' }).trim();
const [version, mode = '--check'] = process.argv.slice(2);
if (!/^v?\d+\.\d+\.\d+$/.test(version ?? '') || !['--check', '--apply'].includes(mode)) {
  console.error('Usage: node scripts/publish-release.mjs <version> [--check|--apply]');
  process.exit(1);
}
const tag = version.startsWith('v') ? version : `v${version}`;
const read = file => readFileSync(join(root, file), 'utf8');
const source = validateReleaseTarget({
  manifest: read('domain.toml'), installSh: read('install.sh'), installPs1: read('install.ps1'),
  origin: run('git', ['remote', 'get-url', 'origin']), mirrorPublisherConfigured: true,
});
const mirror = read('install.sh').match(/^REPO="([^"]+)"/m)[1];
if (mode === '--check') {
  console.log(`${tag}: ${source} -> ${mirror}. Configuration only; nothing published.`);
  process.exit(0);
}
const sha = run('git', ['rev-parse', `${tag}^{commit}`]);
const packageVersion = JSON.parse(run('git', ['show', `${tag}:package.json`])).version;
if (`v${packageVersion}` !== tag) throw new Error('Tag and package version disagree.');
// The versioned manifest must authorize the same destination as the current one.
const versionedInstaller = run('git', ['show', `${tag}:install.sh`]);
if (versionedInstaller.match(/^REPO="([^"]+)"/m)?.[1] !== mirror) throw new Error('Versioned public target differs from current target.');
validateReleaseTarget({
  manifest: run('git', ['show', `${tag}:domain.toml`]),
  installSh: run('git', ['show', `${tag}:install.sh`]),
  installPs1: run('git', ['show', `${tag}:install.ps1`]),
  origin: run('git', ['remote', 'get-url', 'origin']), mirrorPublisherConfigured: true,
});
let runs = [];
for (let attempt = 0; attempt < 6 && !runs.length; attempt++) {
  runs = JSON.parse(run('gh', ['run', 'list', '-R', source, '--workflow', 'release-rust.yml', '--commit', sha, '--event', 'push', '--limit', '1', '--json', 'databaseId,status,conclusion']));
  if (!runs.length) await new Promise(resolve => setTimeout(resolve, 5000));
}
if (!runs.length) throw new Error('Release CI not found yet. Rerun this completion command later.');
if (runs[0].status !== 'completed') run('gh', ['run', 'watch', String(runs[0].databaseId), '-R', source, '--exit-status']);
else if (runs[0].conclusion !== 'success') throw new Error(`Release CI did not succeed: ${runs[0].conclusion}`);

const staging = mkdtempSync(join(tmpdir(), 'cv-public-release-'));
try {
  run('gh', ['release', 'download', tag, '-R', source, '--dir', staging, '--pattern', 'context-vault-*', '--pattern', 'SHA256SUMS']);
  const assets = verifyReleaseAssets(staging).map(file => join(staging, file));
  // This is the only public-tree write path: allowlist + confcheck, committed tag only.
  run('scaffold', ['publish', root, '--rev', tag]);
  const notes = join(staging, 'notes.md');
  writeFileSync(notes, JSON.parse(run('gh', ['release', 'view', tag, '-R', source, '--json', 'body'])).body);
  let existing = false;
  try { run('gh', ['release', 'view', tag, '-R', mirror]); existing = true; } catch { /* create below; permission/network errors still fail */ }
  if (existing) {
    run('gh', ['release', 'upload', tag, '-R', mirror, ...assets, '--clobber']);
    run('gh', ['release', 'edit', tag, '-R', mirror, '--title', tag, '--notes-file', notes, '--latest']);
  }
  else run('gh', ['release', 'create', tag, '-R', mirror, ...assets, '--target', 'main', '--title', tag, '--notes-file', notes, '--latest']);
  console.log(`Published verified public release ${tag} to ${mirror}.`);
} finally {
  rmSync(staging, { recursive: true, force: true });
}
