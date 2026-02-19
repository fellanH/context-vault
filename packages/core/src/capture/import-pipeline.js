/**
 * import-pipeline.js — Batch import orchestrator
 *
 * Processes an array of EntryData through captureAndIndex(),
 * reporting progress and collecting results.
 */

import { captureAndIndex } from "./index.js";
import { indexEntry } from "../index/index.js";

/**
 * @typedef {object} EntryData
 * @property {string} kind
 * @property {string} [title]
 * @property {string} body
 * @property {string[]} [tags]
 * @property {object} [meta]
 * @property {string} [source]
 * @property {string} [identity_key]
 * @property {string} [expires_at]
 */

/**
 * @typedef {object} ImportResult
 * @property {number} imported
 * @property {number} failed
 * @property {Array<{ index: number, title?: string, error: string }>} errors
 */

/**
 * Import an array of entries into the vault.
 *
 * @param {object} ctx — Vault context (db, config, stmts, embed, insertVec, deleteVec)
 * @param {EntryData[]} entries
 * @param {{ onProgress?: (current: number, total: number) => void, source?: string }} [opts]
 * @returns {Promise<ImportResult>}
 */
export async function importEntries(ctx, entries, opts = {}) {
  const { onProgress, source } = opts;
  let imported = 0;
  let failed = 0;
  const errors = [];

  for (let i = 0; i < entries.length; i++) {
    const entry = entries[i];

    if (onProgress) {
      onProgress(i + 1, entries.length);
    }

    try {
      if (!entry.body?.trim()) {
        failed++;
        errors.push({ index: i, title: entry.title, error: "Empty body" });
        continue;
      }

      await captureAndIndex(
        ctx,
        {
          kind: entry.kind || "insight",
          title: entry.title || null,
          body: entry.body,
          meta: entry.meta,
          tags: entry.tags,
          source: entry.source || source || "import",
          identity_key: entry.identity_key,
          expires_at: entry.expires_at,
          userId: ctx.userId || null,
        },
        indexEntry
      );
      imported++;
    } catch (err) {
      failed++;
      errors.push({
        index: i,
        title: entry.title || null,
        error: err.message,
      });
    }
  }

  return { imported, failed, errors };
}
