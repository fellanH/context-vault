/**
 * user-ctx.js — Build per-user vault context from shared server context.
 *
 * In per-user DB mode (PER_USER_DB=true):
 *   Each user gets their own SQLite database, vault directory, and prepared statements
 *   from the UserDbPool. The core package operates within this isolated sandbox
 *   with zero changes — it just sees a different ctx.
 *
 * In legacy mode (PER_USER_DB=false):
 *   Same shared-database behavior as before, with WHERE user_id filtering.
 */

import { encryptForStorage, decryptFromStorage } from "../encryption/vault-crypto.js";
import { getTierLimits } from "../billing/stripe.js";
import { pool, getUserVaultDir, getUserDbPath } from "./user-db.js";
import { PER_USER_DB } from "./ctx.js";

/**
 * Build a per-user context from the shared server context.
 *
 * In per-user mode this is async (pool.get() may initialize a new DB).
 * In legacy mode this is sync-compatible (returns a resolved value).
 *
 * @param {object} ctx — Shared server context
 * @param {{ userId?: string, tier?: string, clientKeyShare?: string } | null} user — Authenticated user info (null for dev mode)
 * @param {string | null} masterSecret — VAULT_MASTER_SECRET env var
 * @param {{ teamId?: string } | null} [teamScope] — Optional team scoping context
 * @returns {Promise<object>} User-scoped context with encrypt/decrypt/checkLimits
 */
export async function buildUserCtx(ctx, user, masterSecret, teamScope) {
  const userId = user?.userId || null;

  if (PER_USER_DB && userId) {
    return buildIsolatedUserCtx(ctx, user, masterSecret, teamScope);
  }

  // Legacy mode (shared database) — sync path
  return buildLegacyUserCtx(ctx, user, masterSecret, teamScope);
}

/**
 * Per-user isolated context — each user gets their own DB, stmts, and vault directory.
 */
async function buildIsolatedUserCtx(ctx, user, masterSecret, teamScope) {
  const userId = user.userId;
  const { db, stmts, insertVec, deleteVec } = await pool.get(userId);

  const vaultDir = getUserVaultDir(userId);
  const dbPath = getUserDbPath(userId);

  const userCtx = {
    db,
    stmts,
    embed: ctx.embed,
    insertVec,
    deleteVec,
    config: {
      ...ctx.config,
      vaultDir,
      dbPath,
      vaultDirExists: true, // we mkdir in pool.get()
    },
    userId,
  };

  // Attach team context when operating in team scope
  if (teamScope?.teamId) {
    userCtx.teamId = teamScope.teamId;
  }

  // Add encryption/decryption when master secret is configured
  if (masterSecret) {
    const clientKeyShare = user.clientKeyShare || null;
    userCtx.encrypt = (entry) => encryptForStorage(entry, userId, masterSecret, clientKeyShare);
    userCtx.decrypt = (row) => decryptFromStorage(row, userId, masterSecret, clientKeyShare);
  }

  // Attach tier limits — queries user's own DB (no WHERE user_id needed)
  if (user.tier) {
    const limits = getTierLimits(user.tier);
    userCtx.checkLimits = () => {
      const entryCount = db.prepare("SELECT COUNT(*) as c FROM vault").get().c;
      const storageBytes = db.prepare(
        "SELECT COALESCE(SUM(LENGTH(COALESCE(body,'')) + LENGTH(COALESCE(body_encrypted,'')) + LENGTH(COALESCE(title,'')) + LENGTH(COALESCE(meta,''))), 0) as s FROM vault"
      ).get().s;
      return {
        entryCount,
        storageMb: storageBytes / (1024 * 1024),
        maxEntries: limits.maxEntries,
        maxStorageMb: limits.storageMb,
      };
    };
  }

  return userCtx;
}

/**
 * Legacy shared-database context — same as before, with WHERE user_id filtering.
 */
function buildLegacyUserCtx(ctx, user, masterSecret, teamScope) {
  const userId = user?.userId || null;
  const userCtx = userId ? { ...ctx, userId } : { ...ctx };

  // Attach team context when operating in team scope
  if (teamScope?.teamId) {
    userCtx.teamId = teamScope.teamId;
  }

  // Add encryption/decryption when master secret is configured and user is authenticated
  if (masterSecret && userId) {
    const clientKeyShare = user?.clientKeyShare || null;
    userCtx.encrypt = (entry) => encryptForStorage(entry, userId, masterSecret, clientKeyShare);
    userCtx.decrypt = (row) => decryptFromStorage(row, userId, masterSecret, clientKeyShare);
  }

  // Attach tier limits for hosted mode
  if (userId && user.tier) {
    const limits = getTierLimits(user.tier);
    userCtx.checkLimits = () => {
      const entryCount = ctx.db.prepare("SELECT COUNT(*) as c FROM vault WHERE user_id = ?").get(userId).c;
      const storageBytes = ctx.db.prepare(
        "SELECT COALESCE(SUM(LENGTH(COALESCE(body,'')) + LENGTH(COALESCE(body_encrypted,'')) + LENGTH(COALESCE(title,'')) + LENGTH(COALESCE(meta,''))), 0) as s FROM vault WHERE user_id = ?"
      ).get(userId).s;
      return {
        entryCount,
        storageMb: storageBytes / (1024 * 1024),
        maxEntries: limits.maxEntries,
        maxStorageMb: limits.storageMb,
      };
    };
  }

  return userCtx;
}
