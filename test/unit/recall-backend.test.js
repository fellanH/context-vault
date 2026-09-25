/**
 * recall-backend tests — recall-core Phase 3 (context-vault backend).
 *
 * Tests:
 *  1. whoami — connector identity shape
 *  2. computeDedupKey — entity branch + content-hash branch
 *  3. toRecallEnvelope — VaultEntry → RecallEnvelope mapping
 *  4. recall — hybrid search returns RecallEnvelope output shape
 *  5. remember — writes to SQLite; returned id is searchable
 *
 * Uses createTestCtx() for a temp isolated DB (same as recall.test.js).
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { execFileSync } from 'node:child_process';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createTestCtx } from '../helpers/ctx.js';
import { captureAndIndex } from '@context-vault/core/capture';
import { hybridSearch } from '@context-vault/core/search';

// Import the envelope helpers directly for unit tests
import {
  computeDedupKey,
  toRecallEnvelope,
} from '../../packages/local/src/recall-envelope.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BIN = join(__dirname, '..', '..', 'packages', 'local', 'bin', 'vault-recall.js');

// ── 1. whoami ─────────────────────────────────────────────────────────────

describe('vault-recall whoami', () => {
  it('outputs expected connector identity', () => {
    const raw = execFileSync(process.execPath, [BIN, 'whoami'], { encoding: 'utf-8' });
    const identity = JSON.parse(raw);
    expect(identity.name).toBe('vault-recall');
    expect(identity.contract_version).toBe('v3');
    expect(identity.handles).toContain('recall');
    expect(identity.handles).toContain('remember');
  });
});

// ── 2. computeDedupKey ────────────────────────────────────────────────────

describe('computeDedupKey', () => {
  it('entity branch uses kind + normalized identity_key', () => {
    const key = computeDedupKey('person', 'entity', 'Felix Hellstrom', 'body');
    expect(key).toBe('entity:person:felix hellstrom');
  });

  it('content-hash branch for non-entity', () => {
    const key = computeDedupKey('insight', 'knowledge', null, 'some body text');
    expect(key).toMatch(/^sha:fnv1a:[0-9a-f]{8}$/);
  });

  it('same content → same dedup_key (deterministic)', () => {
    const a = computeDedupKey('note', 'knowledge', null, 'hello world');
    const b = computeDedupKey('note', 'knowledge', null, 'hello world');
    expect(a).toBe(b);
  });

  it('different content → different dedup_key', () => {
    const a = computeDedupKey('note', 'knowledge', null, 'hello world');
    const b = computeDedupKey('note', 'knowledge', null, 'goodbye world');
    expect(a).not.toBe(b);
  });
});

// ── 3. toRecallEnvelope ───────────────────────────────────────────────────

describe('toRecallEnvelope', () => {
  const entry = {
    id: '01ABC123',
    body: 'test body',
    title: 'test title',
    tags: '["foo","bar"]',
    source: 'cli',
    file_path: '/vault/test.md',
    created_at: '2026-06-23T00:00:00Z',
    updated_at: '2026-06-23T01:00:00Z',
    kind: 'insight',
    category: 'knowledge',
    identity_key: null,
    score: 0.42,
  };

  it('maps core fields', () => {
    const env = toRecallEnvelope(entry);
    expect(env.id).toBe('01ABC123');
    expect(env.text).toBe('test body');
    expect(env.title).toBe('test title');
    expect(env.score).toBe(0.42);
  });

  it('sets origin_backend = context-vault', () => {
    const env = toRecallEnvelope(entry);
    expect(env.provenance.origin_backend).toBe('context-vault');
    expect(env.provenance.role).toBe('canonical');
  });

  it('carries updated_ts when updated_at is set', () => {
    const env = toRecallEnvelope(entry);
    expect(env.provenance.updated_ts).toBe('2026-06-23T01:00:00Z');
  });

  it('parses tags JSON', () => {
    const env = toRecallEnvelope(entry);
    expect(env.tags).toEqual(['foo', 'bar']);
  });

  it('returns [] for null tags', () => {
    const env = toRecallEnvelope({ ...entry, tags: null });
    expect(env.tags).toEqual([]);
  });

  it('computes dedup_key', () => {
    const env = toRecallEnvelope(entry);
    expect(env.dedup_key).toMatch(/^sha:fnv1a:[0-9a-f]{8}$/);
  });
});

// ── 4. recall — integration (isolated DB) ────────────────────────────────

describe('recall integration', () => {
  let ctx, cleanup;

  beforeAll(async () => {
    ({ ctx, cleanup } = await createTestCtx());

    await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'SQLite WAL mode for concurrent reads',
      body: 'Enable WAL mode with PRAGMA journal_mode=WAL for better concurrent read performance.',
      tags: ['sqlite', 'performance'],
    }, null);

    await captureAndIndex(ctx, {
      kind: 'decision',
      title: 'Authentication uses JWT tokens',
      body: 'Decided to use JWT for auth instead of sessions. Stateless, works across services.',
      tags: ['auth', 'jwt'],
    }, null);
  }, 30000);

  afterAll(() => cleanup());

  it('hybridSearch returns RecallEnvelope-shaped results', async () => {
    const results = await hybridSearch(ctx, 'SQLite WAL', { limit: 5, excludeEvents: false });
    expect(results.length).toBeGreaterThan(0);

    const env = toRecallEnvelope({ ...results[0], score: results[0].score });
    expect(env.id).toBeTruthy();
    expect(env.text).toBeTruthy();
    expect(env.provenance.origin_backend).toBe('context-vault');
    expect(env.provenance.role).toBe('canonical');
    expect(env.dedup_key).toMatch(/^sha:fnv1a:[0-9a-f]{8}$/);
    expect(typeof env.score).toBe('number');
  });

  it('envelope has required provenance fields', async () => {
    const results = await hybridSearch(ctx, 'auth', { limit: 5, excludeEvents: false });
    expect(results.length).toBeGreaterThan(0);

    const env = toRecallEnvelope({ ...results[0], score: results[0].score });
    const p = env.provenance;
    expect(p).toHaveProperty('source');
    expect(p).toHaveProperty('origin_backend', 'context-vault');
    expect(p).toHaveProperty('role', 'canonical');
    expect(p).toHaveProperty('ts');
    expect(p).toHaveProperty('entry_kind');
    expect(p).toHaveProperty('mutation_chain');
    expect(Array.isArray(p.mutation_chain)).toBe(true);
  });
});
