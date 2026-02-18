/**
 * vault-crypto.js — Bridge between vault entry format and encrypted DB storage.
 *
 * Connects the entry-level data flow with the low-level crypto primitives
 * (crypto.js) and key management (keys.js).
 */

import { encryptEntry, decryptEntry } from "./crypto.js";
import { getUserDek } from "./keys.js";
import { prepareMetaStatements, getMetaDb } from "../auth/meta-db.js";

/**
 * Encrypt an entry's sensitive fields for database storage.
 *
 * @param {{ title?: string, body: string, meta?: object }} entry
 * @param {string} userId
 * @param {string} masterSecret
 * @returns {{ body_encrypted: Buffer, title_encrypted: Buffer|null, meta_encrypted: Buffer|null, iv: Buffer }}
 */
export function encryptForStorage(entry, userId, masterSecret) {
  const dek = getDekForUser(userId, masterSecret);
  return encryptEntry(entry, dek);
}

/**
 * Decrypt encrypted fields from a database row.
 * Returns plaintext fields. If the row is not encrypted, returns plaintext as-is.
 *
 * @param {{ body_encrypted?: Buffer, title_encrypted?: Buffer, meta_encrypted?: Buffer, iv?: Buffer, body: string, title?: string, meta?: string }} row
 * @param {string} userId
 * @param {string} masterSecret
 * @returns {{ body: string, title: string|null, meta: object|null }}
 */
export function decryptFromStorage(row, userId, masterSecret) {
  if (!row.body_encrypted) {
    // Not encrypted — return plaintext fields
    return {
      body: row.body,
      title: row.title || null,
      meta: row.meta ? (typeof row.meta === "string" ? JSON.parse(row.meta) : row.meta) : null,
    };
  }

  const dek = getDekForUser(userId, masterSecret);
  return decryptEntry(row, dek);
}

/**
 * Get the DEK for a user from the meta DB (with in-memory caching via keys.js).
 */
function getDekForUser(userId, masterSecret) {
  const stmts = prepareMetaStatements(getMetaDb());
  const dekData = stmts.getUserDekData.get(userId);
  if (!dekData?.encrypted_dek || !dekData?.dek_salt) {
    throw new Error(`No encryption key found for user ${userId}. Was the user registered with VAULT_MASTER_SECRET set?`);
  }
  return getUserDek(userId, dekData.encrypted_dek, dekData.dek_salt, masterSecret);
}
