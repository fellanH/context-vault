import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { existsSync } from "node:fs";
import { createTestCtx } from "../helpers/ctx.js";
import { captureAndIndex, updateEntryFile } from "@context-vault/core/capture";
import { indexEntry } from "@context-vault/core/index";
import { hybridSearch } from "@context-vault/core/retrieve";

describe("feedback kind roundtrip", () => {
  let ctx, cleanup;

  beforeAll(async () => {
    ({ ctx, cleanup } = await createTestCtx());
  }, 30000);

  afterAll(() => cleanup());

  let savedId;
  let savedFilePath;

  it("saves a feedback entry with status: new", async () => {
    const entry = await captureAndIndex(
      ctx,
      {
        kind: "feedback",
        title: "Search results missing tags",
        body: "When searching with tag filters, results sometimes omit entries that have matching tags",
        tags: ["bug", "medium"],
        source: "submit_feedback",
        meta: { feedback_type: "bug", severity: "medium", status: "new" },
      },
      indexEntry
    );

    expect(entry.id).toBeTruthy();
    expect(entry.filePath).toBeTruthy();
    expect(existsSync(entry.filePath)).toBe(true);
    expect(entry.kind).toBe("feedback");

    // Verify category is "event" in DB
    const row = ctx.stmts.getEntryById.get(entry.id);
    expect(row.category).toBe("event");

    // Verify meta.status persisted
    const meta = JSON.parse(row.meta);
    expect(meta.status).toBe("new");

    savedId = entry.id;
    savedFilePath = entry.filePath;
  }, 30000);

  it("finds feedback via search", async () => {
    const results = await hybridSearch(ctx, "search results missing tags");
    expect(results.length).toBeGreaterThan(0);
    const found = results.find((r) => r.id === savedId);
    expect(found).toBeTruthy();
    expect(found.kind).toBe("feedback");
    expect(found.category).toBe("event");
  }, 30000);

  it("updates meta.status to processed", async () => {
    const existing = ctx.stmts.getEntryById.get(savedId);
    expect(existing).toBeTruthy();

    const updated = updateEntryFile(ctx, existing, {
      meta: { feedback_type: "bug", severity: "medium", status: "processed" },
    });
    await indexEntry(ctx, updated);

    const row = ctx.stmts.getEntryById.get(savedId);
    const meta = JSON.parse(row.meta);
    expect(meta.status).toBe("processed");
  }, 30000);
});
