#!/usr/bin/env node
// Stage a per-platform npm package for one Rust target triple:
//   dist/npm/<triple>/{package.json, bin/context-vault[.exe]}
// Published as @context-vault/cli-<triple> and referenced by the main package's
// optionalDependencies (esbuild / @biomejs/biome pattern). Part of Phase 7 cutover.

import { mkdirSync, copyFileSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const triple = process.argv[2];
if (!triple) {
  console.error('usage: node scripts/pack-platform.mjs <target-triple>');
  process.exit(1);
}

// Map triple → { os, cpu } for the package.json guards npm uses to pick the right one.
const PLATFORMS = {
  'aarch64-apple-darwin': { os: 'darwin', cpu: 'arm64', exe: '' },
  'x86_64-apple-darwin': { os: 'darwin', cpu: 'x64', exe: '' },
  'aarch64-unknown-linux-gnu': { os: 'linux', cpu: 'arm64', exe: '' },
  'x86_64-unknown-linux-gnu': { os: 'linux', cpu: 'x64', exe: '' },
  'x86_64-pc-windows-msvc': { os: 'win32', cpu: 'x64', exe: '.exe' },
};
const meta = PLATFORMS[triple];
if (!meta) {
  console.error(`unknown target triple: ${triple}`);
  process.exit(1);
}

const version = JSON.parse(readFileSync(join(root, 'packages/local/package.json'), 'utf-8')).version;
const binName = `context-vault${meta.exe}`;
const builtBin = join(root, 'target', triple, 'release', binName);
if (!existsSync(builtBin)) {
  console.error(`built binary not found: ${builtBin} (run cargo build --release --target ${triple})`);
  process.exit(1);
}

const outDir = join(root, 'dist/npm', triple);
mkdirSync(join(outDir, 'bin'), { recursive: true });
copyFileSync(builtBin, join(outDir, 'bin', binName));

const pkg = {
  name: `@context-vault/cli-${triple}`,
  version,
  description: `context-vault native engine binary for ${triple}`,
  license: 'MIT',
  os: [meta.os],
  cpu: [meta.cpu],
  files: ['bin'],
};
writeFileSync(join(outDir, 'package.json'), JSON.stringify(pkg, null, 2) + '\n');
console.log(`staged ${pkg.name}@${version} → ${outDir}`);
