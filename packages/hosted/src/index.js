/**
 * @context-vault/hosted — Hosted context-vault server
 *
 * Hono HTTP server serving MCP over Streamable HTTP transport.
 * Same 6 tools as local mode, shared via registerTools(server, ctx).
 *
 * Stateless per-request model: each request gets a fresh McpServer + transport
 * but shares the same ctx (DB, embeddings, config).
 *
 * Auth modes:
 *   AUTH_REQUIRED=true  → MCP endpoint requires Bearer API key (production)
 *   AUTH_REQUIRED=false → MCP endpoint is open (development, default)
 */

import { Hono } from "hono";
import { cors } from "hono/cors";
import { secureHeaders } from "hono/secure-headers";
import { serve } from "@hono/node-server";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { WebStandardStreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/webStandardStreamableHttp.js";
import { join } from "node:path";
import { writeFileSync, unlinkSync } from "node:fs";
import { registerTools } from "@context-vault/core/server/tools";
import { createCtx } from "./server/ctx.js";
import { initMetaDb, prepareMetaStatements, getMetaDb } from "./auth/meta-db.js";
import { bearerAuth } from "./middleware/auth.js";
import { rateLimit } from "./middleware/rate-limit.js";
import { requestLogger } from "./middleware/logger.js";
import { createManagementRoutes } from "./server/management.js";
import { encryptForStorage, decryptFromStorage } from "./encryption/vault-crypto.js";

// ─── Config ─────────────────────────────────────────────────────────────────

const AUTH_REQUIRED = process.env.AUTH_REQUIRED === "true";

// ─── Startup Validation ─────────────────────────────────────────────────────

const VAULT_MASTER_SECRET = process.env.VAULT_MASTER_SECRET || null;

function validateEnv(config) {
  if (AUTH_REQUIRED) {
    if (!process.env.STRIPE_SECRET_KEY) {
      console.warn("[hosted] \u26a0 STRIPE_SECRET_KEY not set — billing disabled");
    }
    if (!process.env.STRIPE_WEBHOOK_SECRET) {
      console.warn("[hosted] \u26a0 STRIPE_WEBHOOK_SECRET not set — webhooks disabled");
    }
    if (!process.env.STRIPE_PRICE_PRO) {
      console.warn("[hosted] \u26a0 STRIPE_PRICE_PRO not set — checkout disabled");
    }
    if (!VAULT_MASTER_SECRET) {
      console.error("[hosted] FATAL: VAULT_MASTER_SECRET is required when AUTH_REQUIRED=true");
      process.exit(1);
    }
    if (VAULT_MASTER_SECRET.length < 16) {
      console.error("[hosted] FATAL: VAULT_MASTER_SECRET must be at least 16 characters");
      process.exit(1);
    }
  }

  // Verify vault dir is writable
  try {
    const probe = join(config.vaultDir, ".write-test");
    writeFileSync(probe, "");
    unlinkSync(probe);
  } catch (err) {
    console.error(`[hosted] \u26a0 Vault dir not writable: ${config.vaultDir} — ${err.message}`);
  }
}

// ─── Shared Context (initialized once at startup) ───────────────────────────

const ctx = await createCtx();
console.log(`[hosted] Vault: ${ctx.config.vaultDir}`);
console.log(`[hosted] Database: ${ctx.config.dbPath}`);

validateEnv(ctx.config);

// Initialize meta database for auth and usage tracking
const metaDbPath = join(ctx.config.dataDir, "meta.db");
initMetaDb(metaDbPath);
prepareMetaStatements(initMetaDb(metaDbPath));
console.log(`[hosted] Meta DB: ${metaDbPath}`);
console.log(`[hosted] Auth: ${AUTH_REQUIRED ? "required" : "open (dev mode)"}`);

// ─── Factory: create MCP server per request ─────────────────────────────────

function createMcpServer(userId) {
  const server = new McpServer(
    { name: "context-vault-hosted", version: "0.1.0" },
    { capabilities: { tools: {} } }
  );
  // Per-request ctx: shares db/stmts/embed but adds user identity
  // Only set userId when truthy (authenticated) — undefined means local/dev mode (no filtering)
  const userCtx = userId ? { ...ctx, userId } : ctx;

  // Add encryption/decryption functions when master secret is configured and user is authenticated
  if (VAULT_MASTER_SECRET && userId) {
    userCtx.encrypt = (entry) => encryptForStorage(entry, userId, VAULT_MASTER_SECRET);
    userCtx.decrypt = (row) => decryptFromStorage(row, userId, VAULT_MASTER_SECRET);
  }

  registerTools(server, userCtx);
  return server;
}

// ─── Hono App ───────────────────────────────────────────────────────────────

const app = new Hono();

// Global error handler — catches all unhandled errors, returns generic 500
app.onError((err, c) => {
  console.error(JSON.stringify({
    level: "error",
    requestId: c.get("requestId") || null,
    method: c.req.method,
    path: c.req.path,
    error: err.message,
    ts: new Date().toISOString(),
  }));
  return c.json({ error: "Internal server error" }, 500);
});

// 404 handler — JSON instead of Hono's default HTML
app.notFound((c) => c.json({ error: "Not found" }, 404));

// Security headers (X-Content-Type-Options, X-Frame-Options, HSTS, etc.)
app.use("*", secureHeaders());

// Structured JSON request logging
app.use("*", requestLogger());

// CORS for browser-based MCP clients
app.use("*", cors({
  origin: process.env.CORS_ORIGIN || "*",
  allowMethods: ["GET", "POST", "DELETE", "OPTIONS"],
  allowHeaders: ["Content-Type", "Authorization", "mcp-session-id", "Last-Event-ID", "mcp-protocol-version"],
  exposeHeaders: ["mcp-session-id", "mcp-protocol-version", "X-RateLimit-Limit", "X-RateLimit-Remaining", "X-RateLimit-Reset"],
}));

// Health check (unauthenticated) — real DB checks for Fly.io
app.get("/health", (c) => {
  const checks = { status: "ok", version: "0.1.0", auth: AUTH_REQUIRED };
  try { ctx.db.prepare("SELECT 1").get(); checks.vault_db = "ok"; }
  catch { checks.vault_db = "error"; checks.status = "degraded"; }
  try { getMetaDb().prepare("SELECT 1").get(); checks.meta_db = "ok"; }
  catch { checks.meta_db = "error"; checks.status = "degraded"; }
  checks.uptime_s = Math.floor(process.uptime());
  return c.json(checks, checks.status === "ok" ? 200 : 503);
});

// Management REST API (always requires auth)
app.route("/", createManagementRoutes(ctx));

// MCP endpoint — optionally auth-protected
if (AUTH_REQUIRED) {
  app.all("/mcp", bearerAuth(), rateLimit(), async (c) => {
    try {
      const user = c.get("user");
      const transport = new WebStandardStreamableHTTPServerTransport();
      const server = createMcpServer(user.userId);
      await server.connect(transport);
      return transport.handleRequest(c.req.raw);
    } catch (err) {
      console.error(JSON.stringify({
        level: "error",
        requestId: c.get("requestId") || null,
        path: "/mcp",
        error: err.message,
        ts: new Date().toISOString(),
      }));
      return c.json({ error: "Internal server error" }, 500);
    }
  });
} else {
  app.all("/mcp", async (c) => {
    try {
      const transport = new WebStandardStreamableHTTPServerTransport();
      const server = createMcpServer(null);
      await server.connect(transport);
      return transport.handleRequest(c.req.raw);
    } catch (err) {
      console.error(JSON.stringify({
        level: "error",
        requestId: c.get("requestId") || null,
        path: "/mcp",
        error: err.message,
        ts: new Date().toISOString(),
      }));
      return c.json({ error: "Internal server error" }, 500);
    }
  });
}

// ─── Start Server ───────────────────────────────────────────────────────────

const PORT = parseInt(process.env.PORT || "3000", 10);

const httpServer = serve({ fetch: app.fetch, port: PORT }, () => {
  console.log(`[hosted] MCP server listening on http://localhost:${PORT}/mcp`);
  console.log(`[hosted] Health check: http://localhost:${PORT}/health`);
  console.log(`[hosted] Management API: http://localhost:${PORT}/api/*`);
});

// ─── Graceful Shutdown ──────────────────────────────────────────────────────

let shuttingDown = false;
function shutdown(signal) {
  if (shuttingDown) return;
  shuttingDown = true;
  console.log(`[hosted] ${signal} received, draining...`);
  httpServer.close(() => {
    try { ctx.db.close(); } catch {}
    try { getMetaDb().close(); } catch {}
    process.exit(0);
  });
  // Force exit after 10 seconds if drain hangs
  setTimeout(() => { process.exit(1); }, 10_000).unref();
}
process.on("SIGINT", () => shutdown("SIGINT"));
process.on("SIGTERM", () => shutdown("SIGTERM"));
