import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

export const NATIVE_VERSION_FILES = ['crates/context-vault-cli/Cargo.toml', 'Cargo.lock', 'package-lock.json'];

// The native --version, npm launcher and lockfiles must describe the same release.
export function bumpNativeVersion(root, version) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('Invalid release version');
  const patterns = [
    /(\[package\][\s\S]*?\nversion\s*=\s*")[^"]+(")/,
    /(\[\[package\]\]\nname = "context-vault-cli"\nversion = ")[^"]+(")/,
  ];
  const texts = NATIVE_VERSION_FILES.slice(0, 2).map((file, i) => {
    const text = readFileSync(resolve(root, file), 'utf8');
    if (!patterns[i].test(text)) throw new Error(`Missing CLI version in ${file}`);
    return text.replace(patterns[i], (_, prefix, suffix) => `${prefix}${version}${suffix}`);
  });
  const lock = JSON.parse(readFileSync(resolve(root, 'package-lock.json'), 'utf8'));
  if (!lock.packages?.[''] || !lock.packages?.['packages/local']) throw new Error('Missing release workspaces in npm lockfile');
  lock.version = version;
  lock.packages[''].version = version;
  lock.packages['packages/local'].version = version;
  for (const name of Object.keys(lock.packages['packages/local'].optionalDependencies || {})) {
    if (name.startsWith('@context-vault/cli-')) lock.packages['packages/local'].optionalDependencies[name] = version;
  }
  texts.push(JSON.stringify(lock, null, 2) + '\n');
  NATIVE_VERSION_FILES.forEach((file, i) => writeFileSync(resolve(root, file), texts[i]));
}
