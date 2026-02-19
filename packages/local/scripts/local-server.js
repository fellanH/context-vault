#!/usr/bin/env node
/**
 * local-server.js — Local mode: serves app + vault API with no auth.
 *
 * Uses local SQLite vault. No authentication required.
 * Usage: node local-server.js [--port 3141]
 */

import { createServer } from "node:http";
import { createReadStream, existsSync, statSync, unlinkSync } from "node:fs";
import { join, resolve, dirname, extname } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";
import { resolveConfig } from "@context-vault/core/core/config";
import { initDatabase, prepareStatements, insertVec, deleteVec } from "@context-vault/core/index/db";
import { embed } from "@context-vault/core/index/embed";
import { captureAndIndex, updateEntryFile } from "@context-vault/core/capture";
import { indexEntry } from "@context-vault/core/index";
import { hybridSearch } from "@context-vault/core/retrieve";
import { gatherVaultStatus } from "@context-vault/core/core/status";
import { normalizeKind } from "@context-vault/core/core/files";
import { categoryFor } from "@context-vault/core/core/categories";

const __dirname = dirname(fileURLToPath(import.meta.url));
const LOCAL_ROOT = resolve(__dirname, "..");
const APP_DIST = resolve(LOCAL_ROOT, "..", "app", "dist");

const MIME = {
  ".html": "text/html",
  ".js": "application/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".ico": "image/x-icon",
  ".svg": "image/svg+xml",
};

function formatEntry(row) {
  return {
    id: row.id,
    kind: row.kind,
    category: row.category,
    title: row.title || null,
    body: row.body || null,
    tags: row.tags ? JSON.parse(row.tags) : [],
    meta: row.meta ? (typeof row.meta === "string" ? JSON.parse(row.meta) : row.meta) : {},
    source: row.source || null,
    identity_key: row.identity_key || null,
    expires_at: row.expires_at || null,
    created_at: row.created_at,
  };
}

function validateEntry(data, { requireKind = true, requireBody = true } = {}) {
  if (requireKind && !data.kind) return { error: "kind is required", status: 400 };
  if (data.kind && !/^[a-z0-9-]+$/.test(data.kind)) return { error: "kind must be lowercase alphanumeric/hyphens", status: 400 };
  if (requireBody && !data.body) return { error: "body is required", status: 400 };
  if (data.body && data.body.length > 100 * 1024) return { error: "body max 100KB", status: 400 };
  if (categoryFor(data.kind) === "entity" && !data.identity_key) return { error: `Entity kind "${data.kind}" requires identity_key`, status: 400 };
  return null;
}

async function main() {
  const portArg = process.argv.find((a) => a.startsWith("--port="));
  const portVal = portArg ? portArg.split("=")[1] : process.argv[process.argv.indexOf("--port") + 1];
  const port = parseInt(portVal || "3141", 10);

  const config = resolveConfig();
  config.vaultDirExists = existsSync(config.vaultDir);
  let db = await initDatabase(config.dbPath);
  let stmts = prepareStatements(db);

  const state = {
    db,
    config,
    stmts,
    embed,
    get ctx() {
      return {
        db: state.db,
        config: state.config,
        stmts: state.stmts,
        embed: state.embed,
        insertVec: (r, e) => insertVec(state.stmts, r, e),
        deleteVec: (r) => deleteVec(state.stmts, r),
        userId: null,
      };
    },
  };

  const server = createServer(async (req, res) => {
    const url = req.url?.replace(/\?.*$/, "") || "/";

    const json = (data, status = 200) => {
      res.writeHead(status, { "Content-Type": "application/json", "Access-Control-Allow-Origin": "*" });
      res.end(JSON.stringify(data));
    };

    const readBody = () =>
      new Promise((resolve) => {
        let body = "";
        req.on("data", (c) => (body += c));
        req.on("end", () => {
          try {
            resolve(body ? JSON.parse(body) : null);
          } catch {
            resolve(null);
          }
        });
      });

    const ctx = state.ctx;

    // ─── API: POST /api/local/connect — switch to a local vault folder ───────
    if (url === "/api/local/connect" && req.method === "POST") {
      const data = await readBody();
      if (!data?.vaultDir?.trim()) return json({ error: "vaultDir is required", code: "INVALID_INPUT" }, 400);
      let vaultPath = data.vaultDir.trim().replace(/^~/, homedir());
      vaultPath = resolve(vaultPath);
      if (!existsSync(vaultPath)) return json({ error: "Vault folder not found", code: "NOT_FOUND" }, 404);
      if (!statSync(vaultPath).isDirectory()) return json({ error: "Path is not a directory", code: "INVALID_INPUT" }, 400);
      try {
        try { state.db.close(); } catch {}
        const newConfig = { ...state.config, vaultDir: vaultPath, dbPath: join(vaultPath, ".context-vault.db"), vaultDirExists: true };
        state.config = newConfig;
        state.db = await initDatabase(newConfig.dbPath);
        state.stmts = prepareStatements(state.db);
        console.log(`[context-mcp] Switched to vault: ${vaultPath}`);
        return json({
          userId: "local",
          email: "local@localhost",
          name: "Local",
          tier: "free",
          createdAt: new Date().toISOString(),
        });
      } catch (e) {
        console.error(`[local-server] Connect error: ${e.message}`);
        return json({ error: `Failed to connect: ${e.message}`, code: "CONNECT_FAILED" }, 500);
      }
    }

    // ─── API: /api/me (local mode — no auth) ─────────────────────────────────
    if (url === "/api/me" && req.method === "GET") {
      return json({
        userId: "local",
        email: "local@localhost",
        name: "Local",
        tier: "free",
        createdAt: new Date().toISOString(),
      });
    }

    // ─── API: /api/billing/usage (local — unlimited) ─────────────────────────
    if (url === "/api/billing/usage" && req.method === "GET") {
      const status = gatherVaultStatus(ctx, {});
      const total = status.kindCounts.reduce((s, k) => s + k.c, 0);
      const storageMb = Math.round((status.dbSizeBytes / (1024 * 1024)) * 100) / 100;
      return json({
        tier: "free",
        limits: { maxEntries: "unlimited", requestsPerDay: "unlimited", storageMb: 1024, exportEnabled: true },
        usage: { requestsToday: 0, entriesUsed: total, storageMb },
      });
    }

    // ─── API: /api/keys (local — empty) ──────────────────────────────────────
    if (url === "/api/keys" && req.method === "GET") {
      return json({ keys: [] });
    }

    // ─── API: /api/vault/status ─────────────────────────────────────────────
    if (url === "/api/vault/status" && req.method === "GET") {
      const status = gatherVaultStatus(ctx, {});
      return json({
        entries: {
          total: status.kindCounts.reduce((s, k) => s + k.c, 0),
          by_kind: Object.fromEntries(status.kindCounts.map((k) => [k.kind, k.c])),
          by_category: Object.fromEntries(status.categoryCounts.map((k) => [k.category, k.c])),
        },
        files: { total: status.fileCount, directories: status.subdirs },
        database: { size: status.dbSize, size_bytes: status.dbSizeBytes, stale_paths: status.staleCount, expired: status.expiredCount },
        embeddings: status.embeddingStatus,
        embed_model_available: status.embedModelAvailable,
        health: status.errors.length === 0 && !status.stalePaths ? "ok" : "degraded",
        errors: status.errors,
      });
    }

    // ─── API: GET /api/vault/entries ────────────────────────────────────────
    if (url.startsWith("/api/vault/entries") && req.method === "GET") {
      const idMatch = url.match(/\/api\/vault\/entries\/([^/]+)$/);
      if (idMatch) {
        const entry = stmts.getEntryById.get(idMatch[1]);
        if (!entry) return json({ error: "Entry not found", code: "NOT_FOUND" }, 404);
        return json(formatEntry(entry));
      }
      const u = new URL(req.url || "", "http://localhost");
      const kind = u.searchParams.get("kind") || null;
      const category = u.searchParams.get("category") || null;
      const limit = Math.min(parseInt(u.searchParams.get("limit") || "20", 10) || 20, 100);
      const offset = parseInt(u.searchParams.get("offset") || "0", 10) || 0;
      const clauses = ["(expires_at IS NULL OR expires_at > datetime('now'))"];
      const params = [];
      if (kind) {
        clauses.push("kind = ?");
        params.push(normalizeKind(kind));
      }
      if (category) {
        clauses.push("category = ?");
        params.push(category);
      }
      const where = clauses.length ? `WHERE ${clauses.join(" AND ")}` : "";
      const total = ctx.db.prepare(`SELECT COUNT(*) as c FROM vault ${where}`).get(...params).c;
      const rows = ctx.db.prepare(`SELECT * FROM vault ${where} ORDER BY created_at DESC LIMIT ? OFFSET ?`).all(...params, limit, offset);
      return json({ entries: rows.map(formatEntry), total, limit, offset });
    }

    // ─── API: POST /api/vault/entries ────────────────────────────────────────
    if (url === "/api/vault/entries" && req.method === "POST") {
      const data = await readBody();
      if (!data) return json({ error: "Invalid JSON body", code: "INVALID_INPUT" }, 400);
      const err = validateEntry(data);
      if (err) return json({ error: err.error, code: "INVALID_INPUT" }, err.status);
      try {
        const entry = await captureAndIndex(
          ctx,
          {
            kind: data.kind,
            title: data.title,
            body: data.body,
            meta: data.meta,
            tags: data.tags,
            source: data.source || "rest-api",
            identity_key: data.identity_key,
            expires_at: data.expires_at,
            userId: null,
          },
          indexEntry
        );
        return json(formatEntry(stmts.getEntryById.get(entry.id)), 201);
      } catch (e) {
        console.error(`[local-server] Create error: ${e.message}`);
        return json({ error: "Failed to create entry", code: "CREATE_FAILED" }, 500);
      }
    }

    // ─── API: PUT /api/vault/entries/:id ─────────────────────────────────────
    if (url.match(/^\/api\/vault\/entries\/[^/]+$/) && req.method === "PUT") {
      const id = url.split("/").pop();
      const data = await readBody();
      if (!data) return json({ error: "Invalid JSON body", code: "INVALID_INPUT" }, 400);
      const err = validateEntry(data, { requireKind: false, requireBody: false });
      if (err) return json({ error: err.error, code: "INVALID_INPUT" }, err.status);
      const existing = stmts.getEntryById.get(id);
      if (!existing) return json({ error: "Entry not found", code: "NOT_FOUND" }, 404);
      try {
        const entry = updateEntryFile(ctx, existing, {
          title: data.title,
          body: data.body,
          tags: data.tags,
          meta: data.meta,
          source: data.source,
          expires_at: data.expires_at,
        });
        await indexEntry(ctx, entry);
        return json(formatEntry(stmts.getEntryById.get(id)));
      } catch (e) {
        console.error(`[local-server] Update error: ${e.message}`);
        return json({ error: "Failed to update entry", code: "UPDATE_FAILED" }, 500);
      }
    }

    // ─── API: DELETE /api/vault/entries/:id ──────────────────────────────────
    if (url.match(/^\/api\/vault\/entries\/[^/]+$/) && req.method === "DELETE") {
      const id = url.split("/").pop();
      const entry = stmts.getEntryById.get(id);
      if (!entry) return json({ error: "Entry not found", code: "NOT_FOUND" }, 404);
      if (entry.file_path) {
        try {
          unlinkSync(entry.file_path);
        } catch {}
      }
      const rowidResult = stmts.getRowid.get(id);
      if (rowidResult?.rowid) {
        try {
          deleteVec(stmts, Number(rowidResult.rowid));
        } catch {}
      }
      stmts.deleteEntry.run(id);
      return json({ deleted: true, id, kind: entry.kind, title: entry.title || null });
    }

    // ─── API: POST /api/vault/search ─────────────────────────────────────────
    if (url === "/api/vault/search" && req.method === "POST") {
      const data = await readBody();
      if (!data || !data.query?.trim()) return json({ error: "query is required", code: "INVALID_INPUT" }, 400);
      const limit = Math.min(parseInt(data.limit || 20, 10) || 20, 100);
      const offset = parseInt(data.offset || 0, 10) || 0;
      try {
        const results = await hybridSearch(ctx, data.query, {
          kindFilter: data.kind ? normalizeKind(data.kind) : null,
          categoryFilter: data.category || null,
          limit,
          offset,
          decayDays: ctx.config.eventDecayDays || 30,
          userIdFilter: undefined,
        });
        const formatted = results.map((row) => ({ ...formatEntry(row), score: Math.round(row.score * 1000) / 1000 }));
        return json({ results: formatted, count: formatted.length, query: data.query });
      } catch (e) {
        console.error(`[local-server] Search error: ${e.message}`);
        return json({ error: "Search failed", code: "SEARCH_FAILED" }, 500);
      }
    }

    // ─── Static files ───────────────────────────────────────────────────────
    const filePath = join(APP_DIST, url === "/" ? "index.html" : url.replace(/^\//, ""));
    if (!filePath.startsWith(APP_DIST)) {
      res.writeHead(403);
      res.end();
      return;
    }
    if (!existsSync(filePath) || !statSync(filePath).isFile()) {
      const fallback = join(APP_DIST, "index.html");
      if (existsSync(fallback)) {
        res.writeHead(200, { "Content-Type": "text/html" });
        createReadStream(fallback).pipe(res);
        return;
      }
      res.writeHead(404);
      res.end();
      return;
    }
    const type = MIME[extname(filePath)] || "application/octet-stream";
    res.writeHead(200, { "Content-Type": type });
    createReadStream(filePath).pipe(res);
  });

  server.listen(port, () => {
    console.log(`[context-mcp] Local mode: http://localhost:${port}`);
    console.log(`[context-mcp] Vault: ${config.vaultDir}`);
    console.log(`[context-mcp] No authentication required`);
  });

  process.on("SIGINT", () => {
    try { state.db.close(); } catch {}
    process.exit(0);
  });
  process.on("SIGTERM", () => {
    try { state.db.close(); } catch {}
    process.exit(0);
  });
}

main().catch((e) => {
  console.error(`[local-server] Fatal: ${e.message}`);
  process.exit(1);
});
