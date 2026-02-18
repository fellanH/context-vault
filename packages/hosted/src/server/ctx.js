/**
 * ctx.js — Constructs the shared context object for the hosted server.
 *
 * Same ctx shape as local mode, but initialized once at server startup
 * and reused across all HTTP requests.
 */

import { initDatabase, prepareStatements, insertVec, deleteVec } from "@context-vault/core/index/db";
import { embed } from "@context-vault/core/index/embed";
import { resolveConfig } from "@context-vault/core/core/config";
import { existsSync, mkdirSync } from "node:fs";

/**
 * Build the shared ctx object used by all tool handlers.
 * @returns {{ db, config, stmts, embed, insertVec, deleteVec }}
 */
export async function createCtx() {
  const config = resolveConfig();

  // Ensure directories exist
  mkdirSync(config.dataDir, { recursive: true });
  mkdirSync(config.vaultDir, { recursive: true });
  config.vaultDirExists = existsSync(config.vaultDir);

  const db = await initDatabase(config.dbPath);
  const stmts = prepareStatements(db);

  return {
    db,
    config,
    stmts,
    embed,
    insertVec: (rowid, embedding) => insertVec(stmts, rowid, embedding),
    deleteVec: (rowid) => deleteVec(stmts, rowid),
  };
}
