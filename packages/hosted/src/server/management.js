/**
 * management.js — REST API routes for key management, billing, and user operations.
 *
 * Mounted alongside the MCP endpoint in the Hono app.
 */

import { Hono } from "hono";
import { randomUUID } from "node:crypto";
import {
  generateApiKey,
  hashApiKey,
  keyPrefix,
  prepareMetaStatements,
  getMetaDb,
  validateApiKey,
} from "../auth/meta-db.js";

const api = new Hono();

// ─── Auth helper for management routes ──────────────────────────────────────

function requireAuth(c) {
  const header = c.req.header("Authorization");
  if (!header?.startsWith("Bearer ")) return null;
  return validateApiKey(header.slice(7));
}

// ─── API Keys ───────────────────────────────────────────────────────────────

/** List all API keys for the authenticated user */
api.get("/api/keys", (c) => {
  const user = requireAuth(c);
  if (!user) return c.json({ error: "Unauthorized" }, 401);

  const stmts = prepareMetaStatements(getMetaDb());
  const keys = stmts.listUserKeys.all(user.userId);
  return c.json({ keys });
});

/** Create a new API key */
api.post("/api/keys", async (c) => {
  const user = requireAuth(c);
  if (!user) return c.json({ error: "Unauthorized" }, 401);

  const body = await c.req.json().catch(() => ({}));
  const name = body.name || "default";

  const rawKey = generateApiKey();
  const hash = hashApiKey(rawKey);
  const prefix = keyPrefix(rawKey);
  const id = randomUUID();

  const stmts = prepareMetaStatements(getMetaDb());
  stmts.createApiKey.run(id, user.userId, hash, prefix, name);

  // Return the raw key ONCE — it cannot be retrieved again
  return c.json({
    id,
    key: rawKey,
    prefix,
    name,
    message: "Save this key — it will not be shown again.",
  }, 201);
});

/** Delete an API key */
api.delete("/api/keys/:id", (c) => {
  const user = requireAuth(c);
  if (!user) return c.json({ error: "Unauthorized" }, 401);

  const keyId = c.req.param("id");
  const stmts = prepareMetaStatements(getMetaDb());
  const result = stmts.deleteApiKey.run(keyId, user.userId);

  if (result.changes === 0) {
    return c.json({ error: "Key not found" }, 404);
  }
  return c.json({ deleted: true });
});

// ─── User Registration (simplified — no Clerk yet) ─────────────────────────

/** Register a new user and return their first API key */
api.post("/api/register", async (c) => {
  const body = await c.req.json().catch(() => ({}));
  const { email, name } = body;
  if (!email) return c.json({ error: "email is required" }, 400);

  const stmts = prepareMetaStatements(getMetaDb());

  // Check if already exists
  const existing = stmts.getUserByEmail.get(email);
  if (existing) return c.json({ error: "User already exists" }, 409);

  const userId = randomUUID();
  stmts.createUser.run(userId, email, name || null, "free");

  // Generate first API key
  const rawKey = generateApiKey();
  const hash = hashApiKey(rawKey);
  const prefix = keyPrefix(rawKey);
  const keyId = randomUUID();
  stmts.createApiKey.run(keyId, userId, hash, prefix, "default");

  return c.json({
    userId,
    email,
    tier: "free",
    apiKey: {
      id: keyId,
      key: rawKey,
      prefix,
      message: "Save this key — it will not be shown again.",
    },
  }, 201);
});

// ─── Billing ───────────────────────────────────────────────────────────────

import { createCheckoutSession, verifyWebhookEvent, getTierLimits } from "../billing/stripe.js";

api.get("/api/billing/usage", (c) => {
  const user = requireAuth(c);
  if (!user) return c.json({ error: "Unauthorized" }, 401);

  const stmts = prepareMetaStatements(getMetaDb());
  const requestsToday = stmts.countUsageToday.get(user.userId, "mcp_request");
  const limits = getTierLimits(user.tier);

  return c.json({
    tier: user.tier,
    limits: {
      maxEntries: limits.maxEntries === Infinity ? "unlimited" : limits.maxEntries,
      requestsPerDay: limits.requestsPerDay === Infinity ? "unlimited" : limits.requestsPerDay,
      storageMb: limits.storageMb,
      exportEnabled: limits.exportEnabled,
    },
    usage: {
      requestsToday: requestsToday.c,
    },
  });
});

/** Create a Stripe Checkout session for Pro upgrade */
api.post("/api/billing/checkout", async (c) => {
  const user = requireAuth(c);
  if (!user) return c.json({ error: "Unauthorized" }, 401);

  if (user.tier === "pro") {
    return c.json({ error: "Already on Pro tier" }, 400);
  }

  const body = await c.req.json().catch(() => ({}));
  const session = await createCheckoutSession({
    userId: user.userId,
    email: user.email,
    successUrl: body.successUrl,
    cancelUrl: body.cancelUrl,
  });

  if (!session) {
    return c.json({ error: "Stripe not configured. Set STRIPE_SECRET_KEY and STRIPE_PRICE_PRO." }, 503);
  }

  return c.json({ url: session.url, sessionId: session.sessionId });
});

/** Stripe webhook endpoint */
api.post("/api/billing/webhook", async (c) => {
  const body = await c.req.text();
  const signature = c.req.header("stripe-signature");

  if (!signature) return c.json({ error: "Missing stripe-signature" }, 400);

  const event = verifyWebhookEvent(body, signature);
  if (!event) return c.json({ error: "Invalid webhook signature" }, 400);

  const stmts = prepareMetaStatements(getMetaDb());

  switch (event.type) {
    case "checkout.session.completed": {
      const userId = event.data.metadata?.userId;
      if (userId) {
        stmts.updateUserTier.run("pro", userId);
      }
      break;
    }
    case "customer.subscription.deleted": {
      // Downgrade to free on cancellation
      const customerId = event.data.customer;
      if (customerId) {
        // Look up user by stripe customer ID would go here
        // For now, webhook processing is handled
      }
      break;
    }
  }

  return c.json({ received: true });
});

// ─── Vault Import/Export (for migration) ───────────────────────────────────

/** Import a single entry (used by migration CLI) */
api.post("/api/vault/import", async (c) => {
  const user = requireAuth(c);
  if (!user) return c.json({ error: "Unauthorized" }, 401);

  const entry = await c.req.json().catch(() => null);
  if (!entry || !entry.body) return c.json({ error: "Invalid entry: body is required" }, 400);

  // Store directly in vault DB via the shared ctx
  // The ctx is not directly available here, but the MCP tools handle this
  // For now, return the entry as-is for the migration flow to use MCP tools
  return c.json({ received: true, id: entry.id || "pending" });
});

/** Export all entries (used by migration CLI) */
api.get("/api/vault/export", (c) => {
  const user = requireAuth(c);
  if (!user) return c.json({ error: "Unauthorized" }, 401);

  // In production, this would query the user's Turso DB
  // For now, return entries from the shared local vault
  return c.json({ entries: [], message: "Export requires per-user Turso DB (Phase 3+)" });
});

export { api as managementRoutes };
