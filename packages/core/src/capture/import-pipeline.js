import { captureAndIndex } from "./index.js";

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

      await captureAndIndex(ctx, {
        kind: entry.kind || "insight",
        title: entry.title || null,
        body: entry.body,
        meta: entry.meta,
        tags: entry.tags,
        source: entry.source || source || "import",
        identity_key: entry.identity_key,
        expires_at: entry.expires_at,
        userId: ctx.userId || null,
      });
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
