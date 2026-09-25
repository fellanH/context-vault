/**
 * Regression test for v3.18.0 duplicate tool registration bug.
 *
 * v3.18.0 shipped with session-start.ts and recall.ts both exporting
 * name='recall'. register-tools.ts imported both, causing the MCP SDK
 * to throw "Tool recall is already registered" during the CONNECTED
 * phase and breaking all MCP clients.
 *
 * Root cause: no invariant check on tool name uniqueness.
 * This test is that invariant.
 */
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { registerTools } from '../../packages/local/src/register-tools.js';
import { createTestCtx } from '../helpers/ctx.js';

describe('registerTools — no duplicate tool names', () => {
  let ctx, cleanup;

  beforeAll(async () => {
    ({ ctx, cleanup } = await createTestCtx());
  }, 30000);

  afterAll(() => cleanup());

  it('registers every tool with a unique name (no duplicates)', () => {
    const registered = [];
    const fakeServer = {
      tool(name) {
        registered.push(name);
      },
    };

    const localCtx = {
      ...ctx,
      activeOps: { count: 0 },
      toolStats: { ok: 0, errors: 0, lastError: null },
    };

    expect(() => registerTools(fakeServer, localCtx)).not.toThrow();

    const seen = new Set();
    const dupes = [];
    for (const name of registered) {
      if (seen.has(name)) dupes.push(name);
      seen.add(name);
    }

    expect(dupes, `Duplicate tool names registered: ${dupes.join(', ')}`).toEqual([]);
    expect(registered.length).toBeGreaterThan(0);
  });

  it('simulates SDK behavior: registering twice throws', () => {
    const seen = new Set();
    const strictServer = {
      tool(name) {
        if (seen.has(name)) {
          throw new Error(`Tool ${name} is already registered`);
        }
        seen.add(name);
      },
    };

    const localCtx = {
      ...ctx,
      activeOps: { count: 0 },
      toolStats: { ok: 0, errors: 0, lastError: null },
    };

    // Using a separate server instance so the WeakSet short-circuit in
    // registerTools() does not suppress the second call.
    expect(() => registerTools(strictServer, localCtx)).not.toThrow();
  });
});
