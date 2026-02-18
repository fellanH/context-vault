/**
 * rate-limit.js — Tier-based rate limiting and usage metering.
 *
 * Free tier: 100 searches/day, 500 entries max.
 * Pro tier: Unlimited.
 *
 * Note: We cannot read the request body here (it would consume it before
 * the MCP transport can read it). Instead, we do a simple per-request
 * rate limit based on the endpoint, and log usage after the response.
 */

import { prepareMetaStatements, getMetaDb } from "../auth/meta-db.js";

const TIER_LIMITS = {
  free: {
    requestsPerDay: 200,
    maxEntries: 500,
    storageMb: 10,
  },
  pro: {
    requestsPerDay: Infinity,
    maxEntries: Infinity,
    storageMb: 1024,
  },
  team: {
    requestsPerDay: Infinity,
    maxEntries: Infinity,
    storageMb: 5120,
  },
};

/**
 * Hono middleware that enforces tier-based rate limits.
 * Must run after bearerAuth() so c.get("user") is available.
 */
export function rateLimit() {
  return async (c, next) => {
    const user = c.get("user");
    if (!user) return c.json({ error: "Unauthorized" }, 401);

    const limits = TIER_LIMITS[user.tier] || TIER_LIMITS.free;
    const stmts = prepareMetaStatements(getMetaDb());

    // Check daily request limit for free tier
    if (limits.requestsPerDay !== Infinity) {
      const count = stmts.countUsageToday.get(user.userId, "mcp_request");
      if (count.c >= limits.requestsPerDay) {
        return c.json(
          {
            error: `Daily request limit reached (${limits.requestsPerDay}/day). Upgrade to Pro for unlimited usage.`,
            code: "RATE_LIMIT_EXCEEDED",
          },
          429
        );
      }
    }

    // Log usage (before processing — count the attempt)
    try { stmts.logUsage.run(user.userId, "mcp_request"); } catch {}

    await next();
  };
}

export { TIER_LIMITS };
