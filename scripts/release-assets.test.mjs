import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { createHash } from 'node:crypto';
import { RELEASE_ARCHIVES, verifyReleaseAssets } from './release-assets.mjs';

function fixture(t) {
  const directory = mkdtempSync(join(tmpdir(), 'cv-assets-test-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const sum = createHash('sha256').update('fixture').digest('hex');
  for (const file of RELEASE_ARCHIVES) writeFileSync(join(directory, file), 'fixture');
  writeFileSync(join(directory, 'SHA256SUMS'), RELEASE_ARCHIVES.map(f => `${sum}  ${f}`).join('\n'));
  return { directory, sum };
}
test('all five verified platforms and checksums are required', t => {
  const { directory } = fixture(t);
  assert.equal(verifyReleaseAssets(directory).length, 6);
  writeFileSync(join(directory, RELEASE_ARCHIVES[0]), 'corrupt');
  assert.throws(() => verifyReleaseAssets(directory), /Checksum mismatch/);
});
test('partial CI assets are not published', t => {
  const { directory, sum } = fixture(t);
  writeFileSync(join(directory, 'SHA256SUMS'), `${sum}  ${RELEASE_ARCHIVES[0]}`);
  assert.throws(() => verifyReleaseAssets(directory), /Missing release archives/);
});
test('checksum filenames cannot escape the download directory', t => {
  const { directory, sum } = fixture(t);
  writeFileSync(join(directory, 'SHA256SUMS'), `${sum}  ../outside`);
  assert.throws(() => verifyReleaseAssets(directory), /Unexpected checksum entry/);
});
