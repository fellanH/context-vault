/**
 * Unit tests for billing/tier logic.
 */

import { describe, it, expect } from "vitest";
import { getTierLimits, isOverEntryLimit } from "../../packages/hosted/src/billing/stripe.js";

describe("tier limits", () => {
  it("returns free tier limits by default", () => {
    const limits = getTierLimits("free");
    expect(limits.maxEntries).toBe(500);
    expect(limits.storageMb).toBe(10);
    expect(limits.requestsPerDay).toBe(200);
    expect(limits.exportEnabled).toBe(false);
  });

  it("returns pro tier limits", () => {
    const limits = getTierLimits("pro");
    expect(limits.maxEntries).toBe(Infinity);
    expect(limits.storageMb).toBe(1024);
    expect(limits.requestsPerDay).toBe(Infinity);
    expect(limits.exportEnabled).toBe(true);
  });

  it("returns team tier limits", () => {
    const limits = getTierLimits("team");
    expect(limits.maxEntries).toBe(Infinity);
    expect(limits.storageMb).toBe(5120);
  });

  it("defaults to free for unknown tiers", () => {
    const limits = getTierLimits("unknown");
    expect(limits.maxEntries).toBe(500);
  });
});

describe("entry limit enforcement", () => {
  it("free tier is over limit at 500", () => {
    expect(isOverEntryLimit("free", 499)).toBe(false);
    expect(isOverEntryLimit("free", 500)).toBe(true);
    expect(isOverEntryLimit("free", 501)).toBe(true);
  });

  it("pro tier is never over limit", () => {
    expect(isOverEntryLimit("pro", 0)).toBe(false);
    expect(isOverEntryLimit("pro", 100000)).toBe(false);
  });
});
