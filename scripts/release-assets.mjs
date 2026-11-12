import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createHash } from 'node:crypto';

export const RELEASE_ARCHIVES = [
  'aarch64-apple-darwin.tar.gz', 'x86_64-apple-darwin.tar.gz',
  'aarch64-unknown-linux-gnu.tar.gz', 'x86_64-unknown-linux-gnu.tar.gz',
  'x86_64-pc-windows-msvc.zip',
].map(target => `context-vault-${target}`);

/** Refuse missing platforms, unsafe filenames, duplicate entries and corrupt assets. */
export function verifyReleaseAssets(directory) {
  const expected = new Set(RELEASE_ARCHIVES);
  for (const line of readFileSync(join(directory, 'SHA256SUMS'), 'utf8').trim().split('\n')) {
    const match = line.match(/^([a-f0-9]{64})\s+\*?(\S+)$/);
    if (!match || !expected.delete(match[2])) throw new Error(`Unexpected checksum entry: ${line}`);
    const hash = createHash('sha256').update(readFileSync(join(directory, match[2]))).digest('hex');
    if (hash !== match[1]) throw new Error(`Checksum mismatch: ${match[2]}`);
  }
  if (expected.size) throw new Error(`Missing release archives: ${[...expected].join(', ')}`);
  return [...RELEASE_ARCHIVES, 'SHA256SUMS'];
}
