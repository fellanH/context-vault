/**
 * sync.js — Bidirectional sync protocol
 *
 * v1 design:
 * - Additive-only — no delete propagation (avoids data loss)
 * - Last-write-wins by created_at for conflicts (both have same ID)
 * - Push uses POST /api/vault/import/bulk
 * - Pull uses GET /api/vault/export + local captureAndIndex()
 */

import { captureAndIndex } from "../capture/index.js";
import { indexEntry } from "../index/index.js";

/**
 * Build a manifest of local vault entries (id → { id, created_at, kind, title }).
 *
 * @param {import('../server/types.js').BaseCtx} ctx
 * @returns {Map<string, { id: string, created_at: string, kind: string, title: string|null }>}
 */
export function buildLocalManifest(ctx) {
  const rows = ctx.db
    .prepare("SELECT id, created_at, kind, title FROM vault WHERE (expires_at IS NULL OR expires_at > datetime('now'))")
    .all();

  const manifest = new Map();
  for (const row of rows) {
    manifest.set(row.id, {
      id: row.id,
      created_at: row.created_at,
      kind: row.kind,
      title: row.title || null,
    });
  }
  return manifest;
}

/**
 * Fetch the remote vault manifest from the hosted API.
 *
 * @param {string} hostedUrl - Base URL of hosted service
 * @param {string} apiKey - Bearer token
 * @returns {Promise<Map<string, { id: string, created_at: string, kind: string, title: string|null }>>}
 */
export async function fetchRemoteManifest(hostedUrl, apiKey) {
  const response = await fetch(`${hostedUrl}/api/vault/manifest`, {
    headers: { Authorization: `Bearer ${apiKey}` },
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch remote manifest: HTTP ${response.status}`);
  }

  const data = await response.json();
  const manifest = new Map();

  for (const entry of data.entries || []) {
    manifest.set(entry.id, {
      id: entry.id,
      created_at: entry.created_at,
      kind: entry.kind,
      title: entry.title || null,
    });
  }

  return manifest;
}

/**
 * @typedef {object} SyncPlan
 * @property {string[]} toPush - Entry IDs that exist locally but not remotely
 * @property {string[]} toPull - Entry IDs that exist remotely but not locally
 * @property {string[]} upToDate - Entry IDs that exist in both
 */

/**
 * Compute what needs to be pushed/pulled by comparing manifests.
 * Additive-only: entries in both are considered up-to-date.
 *
 * @param {Map<string, object>} local
 * @param {Map<string, object>} remote
 * @returns {SyncPlan}
 */
export function computeSyncPlan(local, remote) {
  const toPush = [];
  const toPull = [];
  const upToDate = [];

  // Find local-only entries
  for (const id of local.keys()) {
    if (remote.has(id)) {
      upToDate.push(id);
    } else {
      toPush.push(id);
    }
  }

  // Find remote-only entries
  for (const id of remote.keys()) {
    if (!local.has(id)) {
      toPull.push(id);
    }
  }

  return { toPush, toPull, upToDate };
}

/**
 * Execute a sync plan: push local entries to remote, pull remote entries to local.
 *
 * @param {import('../server/types.js').BaseCtx & Partial<import('../server/types.js').HostedCtxExtensions>} ctx
 * @param {{ hostedUrl: string, apiKey: string, plan: SyncPlan, onProgress?: (phase: string, current: number, total: number) => void }} opts
 * @returns {Promise<{ pushed: number, pulled: number, failed: number, errors: string[] }>}
 */
export async function executeSync(ctx, { hostedUrl, apiKey, plan, onProgress }) {
  let pushed = 0;
  let pulled = 0;
  let failed = 0;
  const errors = [];

  // ── Push: upload local-only entries to remote ──
  if (plan.toPush.length > 0) {
    const BATCH_SIZE = 50;
    const entries = [];

    // Collect full entry data for push
    for (const id of plan.toPush) {
      const row = ctx.stmts.getEntryById.get(id);
      if (!row) continue;

      entries.push({
        kind: row.kind,
        title: row.title || null,
        body: row.body,
        tags: row.tags ? JSON.parse(row.tags) : [],
        meta: row.meta ? JSON.parse(row.meta) : undefined,
        source: row.source || "sync-push",
        identity_key: row.identity_key || undefined,
        expires_at: row.expires_at || undefined,
      });
    }

    // Push in batches
    for (let i = 0; i < entries.length; i += BATCH_SIZE) {
      const batch = entries.slice(i, i + BATCH_SIZE);
      if (onProgress) onProgress("push", i + batch.length, entries.length);

      try {
        const response = await fetch(`${hostedUrl}/api/vault/import/bulk`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${apiKey}`,
          },
          body: JSON.stringify({ entries: batch }),
        });

        if (!response.ok) {
          const errData = await response.json().catch(() => ({}));
          failed += batch.length;
          errors.push(`Push batch failed: HTTP ${response.status} — ${errData.error || "unknown"}`);
          continue;
        }

        const result = await response.json();
        pushed += result.imported || 0;
        failed += result.failed || 0;
        if (result.errors?.length) {
          errors.push(...result.errors);
        }
      } catch (err) {
        failed += batch.length;
        errors.push(`Push batch failed: ${err.message}`);
      }
    }
  }

  // ── Pull: download remote-only entries to local ──
  if (plan.toPull.length > 0) {
    if (onProgress) onProgress("pull", 0, plan.toPull.length);

    try {
      const response = await fetch(`${hostedUrl}/api/vault/export`, {
        headers: { Authorization: `Bearer ${apiKey}` },
      });

      if (!response.ok) {
        throw new Error(`Export failed: HTTP ${response.status}`);
      }

      const data = await response.json();
      const remoteEntries = data.entries || [];

      // Filter to only pull entries we need
      const pullIds = new Set(plan.toPull);
      const entriesToPull = remoteEntries.filter((e) => pullIds.has(e.id));

      for (let i = 0; i < entriesToPull.length; i++) {
        const entry = entriesToPull[i];
        if (onProgress) onProgress("pull", i + 1, entriesToPull.length);

        try {
          await captureAndIndex(
            ctx,
            {
              kind: entry.kind,
              title: entry.title,
              body: entry.body,
              meta: entry.meta && typeof entry.meta === "object" ? entry.meta : undefined,
              tags: Array.isArray(entry.tags) ? entry.tags : undefined,
              source: entry.source || "sync-pull",
              identity_key: entry.identity_key,
              expires_at: entry.expires_at,
              userId: ctx.userId || null,
            },
            indexEntry
          );
          pulled++;
        } catch (err) {
          failed++;
          errors.push(`Pull "${entry.title || entry.id}": ${err.message}`);
        }
      }
    } catch (err) {
      failed += plan.toPull.length;
      errors.push(`Pull failed: ${err.message}`);
    }
  }

  return { pushed, pulled, failed, errors };
}
