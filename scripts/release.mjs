#!/usr/bin/env node

/**
 * Full release script: bump → verify changelog → commit → tag → push
 *
 * Usage:
 *   npm run release -- patch     # 2.4.2 → 2.4.3
 *   npm run release -- minor     # 2.4.2 → 2.5.0
 *   npm run release -- major     # 2.4.2 → 3.0.0
 *   npm run release -- 2.5.0     # explicit version
 */

import { readFileSync, writeFileSync, unlinkSync } from 'fs';
import { resolve, dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { execSync, execFileSync } from 'child_process';
import { tmpdir } from 'os';
import { bumpNativeVersion, NATIVE_VERSION_FILES } from './release-version.mjs';
import { validateReleaseTarget } from './release-preflight.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const PACKAGE_FILES = ['package.json', 'packages/local/package.json'];
const VERSION_FILES = [...PACKAGE_FILES, ...NATIVE_VERSION_FILES];

// --- Helpers ---

function run(cmd) {
  return execSync(cmd, { cwd: root, encoding: 'utf8', stdio: 'pipe' }).trim();
}

/** Extract the changelog section for a given version */
function extractChangelogEntry(changelog, version) {
  const start = changelog.indexOf(`## [${version}]`);
  if (start === -1) return `Release ${version}`;
  const rest = changelog.slice(start);
  const nextSection = rest.indexOf('\n## ', 1);
  return (nextSection === -1 ? rest : rest.slice(0, nextSection)).trim();
}

function parseVersion(v) {
  const m = v.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!m) throw new Error(`Invalid version: ${v}`);
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

function bump(current, type) {
  const [major, minor, patch] = parseVersion(current);
  switch (type) {
    case 'major':
      return `${major + 1}.0.0`;
    case 'minor':
      return `${major}.${minor + 1}.0`;
    case 'patch':
      return `${major}.${minor}.${patch + 1}`;
    default:
      parseVersion(type); // validate
      return type;
  }
}

// --- Preflight checks ---

const arg = process.argv[2];
if (!arg) {
  console.error('Usage: npm run release -- <patch|minor|major|x.y.z>');
  process.exit(1);
}

// Refuse destination drift before version edits, tags, or publishing.
let releaseSource;
try {
  releaseSource = validateReleaseTarget({
    manifest: readFileSync(resolve(root, 'domain.toml'), 'utf8'),
    installSh: readFileSync(resolve(root, 'install.sh'), 'utf8'),
    installPs1: readFileSync(resolve(root, 'install.ps1'), 'utf8'),
    origin: run('git remote get-url origin'),
    mirrorPublisherConfigured: true,
  });
} catch (error) {
  console.error(`Release preflight refused: ${error.message}`);
  process.exit(1);
}
if (arg === '--check') {
  console.log('Release destinations match. No changes made.');
  process.exit(0);
}

const dirty = run('git status --porcelain');
if (dirty) {
  console.error('Working tree is dirty. Commit or stash changes first.\n');
  console.error(dirty);
  process.exit(1);
}

const branch = run('git rev-parse --abbrev-ref HEAD');
if (branch !== 'main') {
  console.error(`On branch "${branch}" — releases must be from main.`);
  process.exit(1);
}

// Gate the export before any release mutation. No public push occurs here.
execFileSync('scaffold', ['publish', root, '--rev', 'HEAD', '--dry-run'], { cwd: root, stdio: 'inherit' });

// --- Bump versions ---

const rootPkg = JSON.parse(readFileSync(resolve(root, 'package.json'), 'utf8'));
const oldVersion = rootPkg.version;
const newVersion = bump(oldVersion, arg);

console.log(`\n  ${oldVersion} → ${newVersion}\n`);

for (const file of PACKAGE_FILES) {
  const fullPath = resolve(root, file);
  const pkg = JSON.parse(readFileSync(fullPath, 'utf8'));
  pkg.version = newVersion;

  // Pin the per-platform Rust binary packages (@context-vault/cli-<triple>) to the
  // same version — they're published in lockstep by release-rust.yml.
  if (pkg.optionalDependencies) {
    for (const dep of Object.keys(pkg.optionalDependencies)) {
      if (dep.startsWith('@context-vault/cli-')) {
        pkg.optionalDependencies[dep] = newVersion;
      }
    }
  }

  writeFileSync(fullPath, JSON.stringify(pkg, null, 2) + '\n');
  console.log(`  bumped ${file}`);
}

bumpNativeVersion(root, newVersion);

// --- Verify CHANGELOG ---

const changelog = readFileSync(resolve(root, 'CHANGELOG.md'), 'utf8');
if (!changelog.includes(`## [${newVersion}]`)) {
  console.error(`\n  CHANGELOG.md has no entry for [${newVersion}].`);
  console.error(`  Add a "## [${newVersion}]" section before releasing.\n`);
  // Revert bumped files
  run('git checkout -- ' + VERSION_FILES.join(' '));
  console.error('  Reverted version bumps.');
  process.exit(1);
}

// --- Run tests (Rust engine is the source of truth post-migration) ---

console.log('\n  running cargo test...');
run('cargo test --workspace --locked');
console.log('  tests passed\n');

// --- Build the native engine (sanity; CI cross-compiles per-platform) ---

console.log('  building release binary...');
run('cargo build --release -p context-vault-cli --locked');
console.log('  build passed\n');

// --- Commit, tag, push ---
//
// Publishing is delegated to CI: pushing the v<version> tag triggers
// .github/workflows/release-rust.yml, which cross-compiles every target,
// publishes the @context-vault/cli-<triple> platform packages, then publishes the
// main `context-vault` package. See project-management/docs/rust-cutover-runbook.md.

run(`git add ${VERSION_FILES.join(' ')} CHANGELOG.md`);
run(`git commit -m "v${newVersion}"`);
run(`git tag v${newVersion}`);

console.log(`  committed and tagged v${newVersion}`);
console.log(`  pushing to origin (the tag triggers the release-rust CI publish)...\n`);

run('git push origin main --tags');

// --- Create GitHub Release ---

const notes = extractChangelogEntry(changelog, newVersion);
const notesFile = join(tmpdir(), `cv-release-notes-${newVersion}.md`);
writeFileSync(notesFile, notes);
try {
  try {
    execFileSync('gh', ['release', 'create', `v${newVersion}`, '-R', releaseSource,
      '--title', `v${newVersion}`, '--notes-file', notesFile, '--latest'], { cwd: root, stdio: 'inherit' });
  } catch {
    // CI can win the race and create the release first; update its notes in that case.
    execFileSync('gh', ['release', 'edit', `v${newVersion}`, '-R', releaseSource,
      '--title', `v${newVersion}`, '--notes-file', notesFile], { cwd: root, stdio: 'inherit' });
  }
} finally {
  unlinkSync(notesFile);
}

// Wait for CI, gate the public export, then copy checksum-verified assets.
// On failure the private tag remains; retry this idempotent completion command.
console.log(`  Completing public release; retry if needed: node scripts/publish-release.mjs ${newVersion} --apply`);
execFileSync(process.execPath, ['scripts/publish-release.mjs', newVersion, '--apply'], { cwd: root, stdio: 'inherit' });

console.log(`  done.\n`);
