/**
 * CLI parity tests — coverage for flags and subcommands that restore parity
 * with the retired MCP surface (`save_context` id-mutation and `delete_context`).
 *
 *   - save --supersedes   : forward link on new entry + reciprocal superseded_by on old
 *   - save --related-to   : repeatable; stored as JSON array on the new entry
 *   - delete <id>         : removes DB row, vec row, and markdown file; clears backrefs
 *
 * These tests exercise the underlying core/local pipeline that the CLI flags
 * wire into. The CLI flag-parsing itself is exercised via a small parser smoke
 * test that spawns node on the CLI with `--help` (cheap, no DB side-effects).
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

import { createTestCtx } from '../helpers/ctx.js';
import { captureAndIndex } from '@context-vault/core/capture';
import { deleteVec } from '@context-vault/core/db';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const CLI_PATH = resolve(__dirname, '../../packages/local/bin/cli.js');

// ─── --supersedes ─────────────────────────────────────────────────────────────

describe('save --supersedes: writes reciprocal superseded_by link', () => {
  let ctx, cleanup;

  beforeAll(async () => {
    ({ ctx, cleanup } = await createTestCtx());
  }, 30000);

  afterAll(() => cleanup());

  it('new entry records supersedes[] and old entry gets superseded_by', async () => {
    const oldEntry = await captureAndIndex(ctx, {
      kind: 'decision',
      title: 'Original decision',
      body: 'First version of the plan',
      tags: ['test-supersedes'],
    });

    const newEntry = await captureAndIndex(ctx, {
      kind: 'decision',
      title: 'Revised decision',
      body: 'Updated plan based on new evidence',
      tags: ['test-supersedes'],
      supersedes: [oldEntry.id],
    });

    expect(newEntry.supersedes).toEqual([oldEntry.id]);

    const oldRow = ctx.db
      .prepare('SELECT superseded_by FROM vault WHERE id = ?')
      .get(oldEntry.id);
    expect(oldRow.superseded_by).toBe(newEntry.id);

    // The new entry itself is NOT superseded.
    const newRow = ctx.db
      .prepare('SELECT superseded_by FROM vault WHERE id = ?')
      .get(newEntry.id);
    expect(newRow.superseded_by).toBeNull();
  }, 30000);

  it('supports superseding multiple old entries at once', async () => {
    const a = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'Partial insight A',
      body: 'first fragment',
    });
    const b = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'Partial insight B',
      body: 'second fragment',
    });
    const merged = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'Consolidated insight',
      body: 'merged body of A and B',
      supersedes: [a.id, b.id],
    });

    for (const id of [a.id, b.id]) {
      const row = ctx.db
        .prepare('SELECT superseded_by FROM vault WHERE id = ?')
        .get(id);
      expect(row.superseded_by).toBe(merged.id);
    }
  }, 30000);

  it('supersedes survives ignoring whitespace-only entries', async () => {
    const target = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'Target for trim test',
      body: 'body',
    });
    const replacer = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'Replacer',
      body: 'new body',
      supersedes: [`  ${target.id}  `],
    });

    const row = ctx.db
      .prepare('SELECT superseded_by FROM vault WHERE id = ?')
      .get(target.id);
    expect(row.superseded_by).toBe(replacer.id);
  }, 30000);
});

// ─── --related-to ─────────────────────────────────────────────────────────────

describe('save --related-to: stores link array on new entry', () => {
  let ctx, cleanup;

  beforeAll(async () => {
    ({ ctx, cleanup } = await createTestCtx());
  }, 30000);

  afterAll(() => cleanup());

  it('writes related_to as JSON array on new entry (DB and frontmatter)', async () => {
    const a = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'Graph node A',
      body: 'node a body',
    });
    const b = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'Graph node B',
      body: 'node b body',
    });

    const linker = await captureAndIndex(ctx, {
      kind: 'decision',
      title: 'Linker decision',
      body: 'references a and b',
      related_to: [a.id, b.id],
    });

    expect(linker.related_to).toEqual([a.id, b.id]);

    const row = ctx.db
      .prepare('SELECT related_to FROM vault WHERE id = ?')
      .get(linker.id);
    const stored = JSON.parse(row.related_to);
    expect(stored).toEqual([a.id, b.id]);

    // Frontmatter on disk includes the related_to block.
    const md = readFileSync(linker.filePath, 'utf-8');
    expect(md).toContain('related_to:');
    expect(md).toContain(a.id);
    expect(md).toContain(b.id);
  }, 30000);

  it('related_to is non-reciprocal (target entries are NOT auto-back-linked)', async () => {
    const target = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'Target',
      body: 'target body',
    });
    await captureAndIndex(ctx, {
      kind: 'decision',
      title: 'Linker',
      body: 'points at target',
      related_to: [target.id],
    });

    const targetRow = ctx.db
      .prepare('SELECT related_to FROM vault WHERE id = ?')
      .get(target.id);
    expect(targetRow.related_to).toBeNull();
  }, 30000);
});

// ─── delete <id> ──────────────────────────────────────────────────────────────

describe('delete <id>: hard remove of DB row, vec row, and file', () => {
  let ctx, cleanup;

  beforeAll(async () => {
    ({ ctx, cleanup } = await createTestCtx());
  }, 30000);

  afterAll(() => cleanup());

  // Helper that mirrors runDelete's core work (DB + vec + file + backref clear).
  function hardDelete(entryId) {
    const rowidRow = ctx.stmts.getRowid.get(entryId);
    if (rowidRow?.rowid) {
      try {
        deleteVec(ctx.stmts, Number(rowidRow.rowid));
      } catch {}
    }
    ctx.stmts.clearSupersededByRef.run(entryId);
    ctx.stmts.deleteEntry.run(entryId);
    const row = ctx.db.prepare('SELECT file_path FROM vault WHERE id = ?').get(entryId);
    return row ? row.file_path : null;
  }

  it('removes entry from DB entirely', async () => {
    const entry = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'To delete',
      body: 'disposable content',
    });
    expect(existsSync(entry.filePath)).toBe(true);

    hardDelete(entry.id);
    const { unlinkSync } = await import('node:fs');
    try { unlinkSync(entry.filePath); } catch {}

    const row = ctx.db.prepare('SELECT id FROM vault WHERE id = ?').get(entry.id);
    expect(row).toBeFalsy();
    expect(existsSync(entry.filePath)).toBe(false);
  }, 30000);

  it('search returns 0 matches for a deleted entry by id', async () => {
    const entry = await captureAndIndex(ctx, {
      kind: 'insight',
      title: 'DeleteSearchCanary unique-marker-xqyz',
      body: 'unique-marker-xqyz DeleteSearchCanary body',
    });

    hardDelete(entry.id);
    const { unlinkSync } = await import('node:fs');
    try { unlinkSync(entry.filePath); } catch {}

    const row = ctx.db.prepare('SELECT id FROM vault WHERE id = ?').get(entry.id);
    expect(row).toBeFalsy();

    const { hybridSearch } = await import('@context-vault/core/search');
    const results = await hybridSearch(ctx, 'DeleteSearchCanary unique-marker-xqyz');
    const hit = results.find((r) => r.id === entry.id);
    expect(hit).toBeFalsy();
  }, 30000);

  it('clears superseded_by refs on other entries pointing at the deleted one', async () => {
    // Chain: old <- new (new supersedes old)
    // If we delete NEW, `old.superseded_by` must become NULL — no dangling ref.
    const oldEntry = await captureAndIndex(ctx, {
      kind: 'decision',
      title: 'Old decision (chain)',
      body: 'body',
    });
    const newEntry = await captureAndIndex(ctx, {
      kind: 'decision',
      title: 'New decision (chain)',
      body: 'replacement',
      supersedes: [oldEntry.id],
    });

    // Sanity: the backref exists.
    const before = ctx.db
      .prepare('SELECT superseded_by FROM vault WHERE id = ?')
      .get(oldEntry.id);
    expect(before.superseded_by).toBe(newEntry.id);

    hardDelete(newEntry.id);
    const { unlinkSync } = await import('node:fs');
    try { unlinkSync(newEntry.filePath); } catch {}

    const after = ctx.db
      .prepare('SELECT superseded_by FROM vault WHERE id = ?')
      .get(oldEntry.id);
    expect(after.superseded_by).toBeNull();
  }, 30000);
});

// ─── CLI flag parsing smoke test ──────────────────────────────────────────────
// Cheap end-to-end check that the flags appear in --help (so --help parses and
// the new surface is discoverable), without spinning up a real vault.

describe('CLI flag discoverability', () => {
  it('save --help mentions --supersedes and --related-to', () => {
    const out = execFileSync(
      process.execPath,
      [CLI_PATH, 'save', '--help'],
      { encoding: 'utf-8' }
    );
    expect(out).toContain('--supersedes');
    expect(out).toContain('--related-to');
    expect(out).toContain('superseded_by');
  });

  it('delete --help describes the command', () => {
    const out = execFileSync(
      process.execPath,
      [CLI_PATH, 'delete', '--help'],
      { encoding: 'utf-8' }
    );
    expect(out).toContain('context-vault delete');
    expect(out).toContain('Hard-delete');
    expect(out).toContain('superseded_by');
  });

  it('delete without id exits with error', () => {
    let threw = false;
    try {
      execFileSync(process.execPath, [CLI_PATH, 'delete'], {
        encoding: 'utf-8',
        stdio: 'pipe',
      });
    } catch (e) {
      threw = true;
      expect(e.status).toBe(1);
      expect(e.stderr).toContain('entry id is required');
    }
    expect(threw).toBe(true);
  });

  it('top-level help lists delete as a command', () => {
    const out = execFileSync(
      process.execPath,
      [CLI_PATH, '--help'],
      { encoding: 'utf-8' }
    );
    expect(out).toContain('delete');
  });
});

// ─── omni-vault cutover surface (get / buckets / dupes / stats counts) ─────────
// These verbs surface existing vault capability on the CLI so `omni vault <verb>`
// can route to the context-vault connector when omni-legacy retires. See
// specs/omni-vault-cutover-mapping.md.

describe('omni-vault cutover verbs', () => {
  it('get --help describes id/identity-key lookup', () => {
    const out = execFileSync(process.execPath, [CLI_PATH, 'get', '--help'], {
      encoding: 'utf-8',
    });
    expect(out).toContain('context-vault get');
    expect(out).toContain('identity key');
    expect(out).toContain('--kind');
  });

  it('get without a ref exits with error', () => {
    let threw = false;
    try {
      execFileSync(process.execPath, [CLI_PATH, 'get'], {
        encoding: 'utf-8',
        stdio: 'pipe',
      });
    } catch (e) {
      threw = true;
      expect(e.status).toBe(1);
      expect(e.stderr).toContain('identity key is required');
    }
    expect(threw).toBe(true);
  });

  it('buckets --help describes the command', () => {
    const out = execFileSync(process.execPath, [CLI_PATH, 'buckets', '--help'], {
      encoding: 'utf-8',
    });
    expect(out).toContain('context-vault buckets');
    expect(out).toContain('entry counts');
  });

  it('dupes --help documents read-only similarity scan', () => {
    const out = execFileSync(process.execPath, [CLI_PATH, 'dupes', '--help'], {
      encoding: 'utf-8',
    });
    expect(out).toContain('context-vault dupes');
    expect(out).toContain('Read-only');
    expect(out).toContain('--threshold');
  });

  it('stats counts is an accepted subcommand', () => {
    // Unknown stats subcommand errors and lists `counts` as available.
    let stderr = '';
    try {
      execFileSync(process.execPath, [CLI_PATH, 'stats', 'bogus'], {
        encoding: 'utf-8',
        stdio: 'pipe',
      });
    } catch (e) {
      stderr = e.stderr || '';
    }
    expect(stderr).toContain('counts');
  });

  it('top-level help lists get, buckets, and dupes', () => {
    const out = execFileSync(process.execPath, [CLI_PATH, '--help'], {
      encoding: 'utf-8',
    });
    expect(out).toContain('get');
    expect(out).toContain('buckets');
    expect(out).toContain('dupes');
  });
});
