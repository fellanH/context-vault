/**
 * keys.js — Key derivation and DEK (Data Encryption Key) management.
 *
 * Architecture:
 *   user password → scrypt → master_key → encrypts DEK
 *   DEK stored encrypted in meta DB (encrypted_dek + dek_salt columns)
 *   DEK held in memory during active sessions only
 *
 * For the initial implementation, we use a server-managed master key
 * (from environment variable) rather than user passwords.
 * This gives "encrypted at rest" protection without requiring users to
 * manage passwords — same model as most cloud services.
 */

import { scryptSync, randomBytes } from "node:crypto";
import { encrypt, decrypt } from "./crypto.js";

const KEY_LENGTH = 32; // 256 bits for AES-256
const SCRYPT_N = 16384;
const SCRYPT_R = 8;
const SCRYPT_P = 1;

/**
 * Derive a 256-bit key from a password/secret and salt using scrypt.
 *
 * @param {string} secret - Password or master secret
 * @param {Buffer} salt - 16-byte random salt
 * @returns {Buffer} - 32-byte derived key
 */
export function deriveKey(secret, salt) {
  return scryptSync(secret, salt, KEY_LENGTH, { N: SCRYPT_N, r: SCRYPT_R, p: SCRYPT_P });
}

/**
 * Generate a new random DEK and encrypt it with the master key.
 *
 * @param {string} masterSecret - Server master secret (from env)
 * @returns {{ encryptedDek: Buffer, dekSalt: Buffer, dek: Buffer }}
 */
export function generateDek(masterSecret) {
  const dek = randomBytes(KEY_LENGTH);
  const dekSalt = randomBytes(16);
  const masterKey = deriveKey(masterSecret, dekSalt);
  const { encrypted, iv } = encrypt(dek.toString("hex"), masterKey);

  // Store IV + encrypted DEK together
  const encryptedDek = Buffer.concat([iv, encrypted]);

  return { encryptedDek, dekSalt, dek };
}

/**
 * Decrypt a stored DEK using the master key.
 *
 * @param {Buffer} encryptedDek - IV (12 bytes) + encrypted DEK
 * @param {Buffer} dekSalt - Salt used for master key derivation
 * @param {string} masterSecret - Server master secret
 * @returns {Buffer} - 32-byte DEK
 */
export function decryptDek(encryptedDek, dekSalt, masterSecret) {
  const iv = encryptedDek.subarray(0, 12);
  const encrypted = encryptedDek.subarray(12);
  const masterKey = deriveKey(masterSecret, dekSalt);
  const dekHex = decrypt(encrypted, iv, masterKey);
  return Buffer.from(dekHex, "hex");
}

/**
 * In-memory DEK cache keyed by userId.
 * DEKs are cached for the lifetime of the server process.
 */
const dekCache = new Map();

/**
 * Get or derive the DEK for a user.
 * Caches the result in memory.
 *
 * @param {string} userId
 * @param {Buffer} encryptedDek - From meta DB
 * @param {Buffer} dekSalt - From meta DB
 * @param {string} masterSecret - From environment
 * @returns {Buffer} - 32-byte DEK
 */
export function getUserDek(userId, encryptedDek, dekSalt, masterSecret) {
  if (dekCache.has(userId)) return dekCache.get(userId);
  const dek = decryptDek(encryptedDek, dekSalt, masterSecret);
  dekCache.set(userId, dek);
  return dek;
}

/** Clear cached DEK (e.g., on key rotation). */
export function clearDekCache(userId) {
  if (userId) dekCache.delete(userId);
  else dekCache.clear();
}
