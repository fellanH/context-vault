#!/usr/bin/env node
// Complete a private-source release on the public mirror, without changing source remotes.
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdtempSync, rmSync, copyFileSync, existsSync } from 'node:fs';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir, homedir } from 'node:os';
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
// The binaries build on the public mirror, whose Actions runners are free; a free account's
// private minutes ran out on 18 Nov 2026 and refused every build (OPS-482). Order: export the
// tagged tree, create the public release as a pre-release (install.sh keeps reading the previous
// Latest), let the mirror's CI attach the five archives, verify them, and only then mark it Latest.
// This is the only public-tree write path: allowlist + confcheck, committed tag only.
run('scaffold', ['publish', root, '--rev', tag]);
const staging = mkdtempSync(join(tmpdir(), 'cv-public-release-'));
try {
  const notes = join(staging, 'notes.md');
  writeFileSync(notes, JSON.parse(run('gh', ['release', 'view', tag, '-R', source, '--json', 'body'])).body);
  let existing = false;
  try { run('gh', ['release', 'view', tag, '-R', mirror]); existing = true; } catch { /* create below; permission/network errors still fail */ }
  if (existing) run('gh', ['release', 'edit', tag, '-R', mirror, '--title', tag, '--notes-file', notes]);
  else run('gh', ['release', 'create', tag, '-R', mirror, '--target', 'main', '--title', tag, '--notes-file', notes, '--prerelease']);
  const ci = await mirrorReleaseRun(mirror, tag);
  const jobs = JSON.parse(run('gh', ['run', 'view', String(ci), '-R', mirror, '--json', 'jobs'])).jobs;
  const gating = jobs.filter(job => job.name.startsWith('build ') || job.name.startsWith('publish GitHub Release'));
  const failed = gating.filter(job => job.conclusion !== 'success');
  if (!gating.length || failed.length) throw new Error(`Release CI did not succeed on ${mirror}: ${failed.map(job => `${job.name} ${job.conclusion}`).join(', ') || 'no build jobs found'}. ${tag} stays a pre-release with whatever assets CI attached, and the mirror's main already holds the new tree; install.sh still installs the previous Latest. Rerun this command to retry the failed jobs, or delete the pre-release: gh release delete ${tag} -R ${mirror} --cleanup-tag.`);
  run('gh', ['release', 'download', tag, '-R', mirror, '--dir', staging, '--pattern', 'context-vault-*', '--pattern', 'SHA256SUMS', '--clobber']);
  verifyReleaseAssets(staging);
  run('gh', ['release', 'edit', tag, '-R', mirror, '--prerelease=false', '--latest']);
  console.log(`Published verified public release ${tag} to ${mirror}.`);
} finally {
  rmSync(staging, { recursive: true, force: true });
}

/** The mirror's release-rust run for this tag, waited on until it completes. */
async function mirrorReleaseRun(repo, ref) {
  let runs = [];
  for (let attempt = 0; attempt < 24 && !runs.length; attempt++) {
    runs = JSON.parse(run('gh', ['run', 'list', '-R', repo, '--workflow', 'release-rust.yml', '--branch', ref, '--event', 'push', '--limit', '1', '--json', 'databaseId,status,conclusion']));
    if (!runs.length) await new Promise(resolve => setTimeout(resolve, Number(process.env.CV_RELEASE_POLL_MS ?? 5000)));
  }
  if (!runs.length) throw new Error(`Release CI for ${ref} not found on ${repo} yet. Rerun this completion command later.`);
  const id = String(runs[0].databaseId);
  // A rerun of this command after a failed or cancelled build reruns that run's failed jobs,
  // since the tag already exists and pushes no new run.
  if (runs[0].status === 'completed' && runs[0].conclusion !== 'success') run('gh', ['run', 'rerun', id, '--failed', '-R', repo]);
  if (runs[0].status !== 'completed' || runs[0].conclusion !== 'success') {
    execFileSync('gh', ['run', 'watch', id, '-R', repo, '--interval', '30'], { cwd: root, stdio: 'inherit' });
  }
  return id;
}

// The machine that publishes also runs the tool every day, so the release installs here too and
// proves itself: version and doctor. A new version then reaches it with no hand step (OPS-401).
// The previous binary stays beside it as context-vault.previous. CONTEXT_VAULT_SKIP_LOCAL_INSTALL=1 skips this.
if (process.env.CONTEXT_VAULT_SKIP_LOCAL_INSTALL !== '1') {
  const dir = process.env.CONTEXT_VAULT_INSTALL_DIR || join(homedir(), '.local', 'bin');
  const bin = join(dir, 'context-vault');
  if (existsSync(bin)) copyFileSync(bin, `${bin}.previous`);
  let installed = '';
  for (let attempt = 0; attempt < 4 && !installed.endsWith(tag.slice(1)); attempt++) {
    if (attempt) await new Promise(resolve => setTimeout(resolve, 15000)); // releases/latest can lag the edit
    execFileSync('sh', [join(root, 'install.sh')], { cwd: root, stdio: 'inherit' });
    installed = execFileSync(bin, ['--version'], { encoding: 'utf8' }).trim();
  }
  const doctor = execFileSync(bin, ['doctor'], { encoding: 'utf8' });
  if (!installed.endsWith(tag.slice(1)) || !/engine health: OK/.test(doctor)) {
    throw new Error(`Local install did not take (${installed}). Put the previous one back: mv "${bin}.previous" "${bin}"`);
  }
  console.log(`Installed ${installed} on this machine; doctor OK.`);
}
