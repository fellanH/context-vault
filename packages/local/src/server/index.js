#!/usr/bin/env node

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { existsSync, mkdirSync, writeFileSync, readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkg = JSON.parse(readFileSync(join(__dirname, "..", "..", "package.json"), "utf-8"));

import { resolveConfig } from "@context-vault/core/core/config";
import { embed } from "@context-vault/core/index/embed";
import { initDatabase, prepareStatements, insertVec, deleteVec } from "@context-vault/core/index/db";
import { registerTools } from "@context-vault/core/server/tools";

// ─── Config Resolution ──────────────────────────────────────────────────────

const config = resolveConfig();

// Create directories
mkdirSync(config.dataDir, { recursive: true });
mkdirSync(config.vaultDir, { recursive: true });

// Write .context-mcp marker (always update to reflect current version)
const markerPath = join(config.vaultDir, ".context-mcp");
const markerData = existsSync(markerPath) ? JSON.parse(readFileSync(markerPath, "utf-8")) : {};
writeFileSync(markerPath, JSON.stringify({ created: markerData.created || new Date().toISOString(), version: pkg.version }, null, 2) + "\n");

// Update existence flag after directory creation
config.vaultDirExists = existsSync(config.vaultDir);

// Startup diagnostics
console.error(`[context-mcp] Vault: ${config.vaultDir}`);
console.error(`[context-mcp] Database: ${config.dbPath}`);
console.error(`[context-mcp] Dev dir: ${config.devDir}`);
if (!config.vaultDirExists) {
  console.error(`[context-mcp] WARNING: Vault directory not found!`);
}

// ─── Database Init ───────────────────────────────────────────────────────────

let db, stmts;
try {
  db = initDatabase(config.dbPath);
  stmts = prepareStatements(db);
} catch (e) {
  console.error(`[context-mcp] Database init failed: ${e.message}`);
  console.error(`[context-mcp] DB path: ${config.dbPath}`);
  console.error(`[context-mcp] Try deleting the DB file and restarting: rm "${config.dbPath}"`);
  process.exit(1);
}

const ctx = {
  db,
  config,
  stmts,
  embed,
  insertVec: (rowid, embedding) => insertVec(stmts, rowid, embedding),
  deleteVec: (rowid) => deleteVec(stmts, rowid),
};

// ─── MCP Server ──────────────────────────────────────────────────────────────

const server = new McpServer(
  { name: "context-mcp", version: pkg.version },
  { capabilities: { tools: {} } }
);

registerTools(server, ctx);

// ─── Graceful Shutdown ───────────────────────────────────────────────────────

function shutdown() {
  try { db.close(); } catch {}
  process.exit(0);
}
process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);

const transport = new StdioServerTransport();
await server.connect(transport);

// ─── Non-blocking Update Check ──────────────────────────────────────────────

setTimeout(() => {
  import("node:child_process").then(({ execSync }) => {
    try {
      const latest = execSync("npm view context-vault version", {
        encoding: "utf-8",
        timeout: 5000,
        stdio: ["pipe", "pipe", "pipe"],
      }).trim();
      if (latest && latest !== pkg.version) {
        console.error(`[context-mcp] Update available: v${pkg.version} → v${latest}. Run: context-mcp update`);
      }
    } catch {}
  }).catch(() => {});
}, 3000);
