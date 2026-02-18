/**
 * auth.js — Bearer token authentication middleware for Hono.
 *
 * Validates API keys from the Authorization header.
 * Sets c.set("user", { userId, email, tier, ... }) on success.
 */

import { validateApiKey } from "../auth/meta-db.js";

/**
 * Hono middleware that requires a valid API key.
 * Skips auth for health check and other non-MCP routes.
 */
export function bearerAuth() {
  return async (c, next) => {
    const header = c.req.header("Authorization");
    if (!header || !header.startsWith("Bearer ")) {
      return c.json(
        { error: "Missing or invalid Authorization header. Use: Bearer cv_..." },
        401
      );
    }

    const token = header.slice(7);
    const user = validateApiKey(token);
    if (!user) {
      return c.json({ error: "Invalid or expired API key" }, 401);
    }

    // Attach user info to the request context
    c.set("user", user);
    await next();
  };
}
