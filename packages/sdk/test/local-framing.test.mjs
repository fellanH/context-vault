// Regression test for the LocalBackend stdio frame parser.
//
// Guards against the byte-vs-char length bug: Content-Length is a UTF-8 byte
// count, but a JS-string-based parser measures UTF-16 code units, so any
// multibyte character in a reply (e.g. the "✓" the engine returns from
// save_context) leaves the frame permanently "incomplete" and capture()/save()
// hangs forever. This drives the public save() API against a fake server whose
// reply contains "✓" and asserts it resolves (does not hang) with the ULID.
//
// Dependency-free: runs under Node's built-in test runner.
//   node --test packages/sdk/test/
// Requires the SDK to be built first (imports the shipped dist artifact):
//   npx tsc -p packages/sdk/tsconfig.json

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { LocalBackend } from '../dist/local.js';

const here = dirname(fileURLToPath(import.meta.url));
const fakeServer = join(here, 'fixtures', 'multibyte-mcp-server.mjs');

test('save() resolves when the reply contains a multibyte character', async () => {
  // bin points at the fake server; LocalBackend spawns it as `<bin> mcp`.
  // The fixture ignores argv, so the extra `mcp` arg is harmless.
  const backend = new LocalBackend({ bin: fakeServer });

  try {
    // If the byte-vs-char bug regresses, this never resolves and the test times
    // out (a failure) instead of returning.
    const result = await backend.save({
      kind: 'note',
      title: 'multibyte framing probe',
      body: 'hello',
      tier: 'working',
    });

    assert.equal(result.id, '01KWVHGN6NC3JQX5K02Q752ABC');
    assert.equal(result.freshness.label, 'fresh');
  } finally {
    // Kill the spawned fake server so the test runner's event loop can exit.
    await backend.close();
  }
});
