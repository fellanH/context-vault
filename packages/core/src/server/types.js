/**
 * types.js — JSDoc type definitions for ctx shapes across modes.
 *
 * These typedefs document the context object passed through tool handlers,
 * index, retrieve, and sync layers.
 */

/**
 * @typedef {Object} VaultConfig
 * @property {string} vaultDir - Root vault directory path
 * @property {string} dbPath - SQLite database file path
 * @property {string} dataDir - Data directory for embeddings, models, etc.
 * @property {string} devDir - Dev/config directory
 * @property {number} eventDecayDays - Recency decay window for event entries
 * @property {string} [configPath] - Path to resolved config file
 * @property {boolean} [vaultDirExists] - Whether the vault directory exists on disk
 * @property {string} [resolvedFrom] - How the config was resolved (env, flag, default, etc.)
 */

/**
 * @typedef {Object} PreparedStatements
 * @property {import('better-sqlite3').Statement} insertEntry
 * @property {import('better-sqlite3').Statement} insertEntryEncrypted
 * @property {import('better-sqlite3').Statement} updateEntry
 * @property {import('better-sqlite3').Statement} upsertByIdentityKey
 * @property {import('better-sqlite3').Statement} getEntryById
 * @property {import('better-sqlite3').Statement} getByIdentityKey
 * @property {import('better-sqlite3').Statement} getRowid
 * @property {import('better-sqlite3').Statement} getRowidByPath
 * @property {import('better-sqlite3').Statement} deleteEntry
 */

/**
 * Core context properties present in all modes.
 *
 * @typedef {Object} BaseCtx
 * @property {import('better-sqlite3').Database} db - SQLite database instance
 * @property {VaultConfig} config - Resolved vault configuration
 * @property {PreparedStatements} stmts - Prepared SQL statements
 * @property {(text: string) => Promise<Float32Array|null>} embed - Generate embedding vector for text
 * @property {(rowid: number, embedding: Float32Array) => void} insertVec - Insert vector embedding
 * @property {(rowid: number) => void} deleteVec - Delete vector embedding
 */

/**
 * Context for local (CLI/MCP) mode. Extends BaseCtx with graceful shutdown tracking.
 *
 * @typedef {BaseCtx & LocalCtxExtensions} LocalCtx
 */

/**
 * @typedef {Object} LocalCtxExtensions
 * @property {{ count: number }} [activeOps] - In-flight operation counter for graceful shutdown
 */

/**
 * Context for hosted (multi-tenant) mode. Extends BaseCtx with auth and encryption.
 *
 * @typedef {BaseCtx & HostedCtxExtensions} HostedCtx
 */

/**
 * @typedef {Object} HostedCtxExtensions
 * @property {string} userId - Authenticated user ID
 * @property {string} [teamId] - Team ID for team-scoped vaults
 * @property {{ count: number }} [activeOps] - In-flight operation counter for graceful shutdown
 * @property {(entry: { title?: string, body: string, meta?: object }) => Promise<{ body_encrypted: string, title_encrypted?: string, meta_encrypted?: string, iv: string }>} [encrypt] - Encrypt entry fields
 * @property {(row: { body_encrypted: string, title_encrypted?: string, meta_encrypted?: string, iv: string }) => Promise<{ body: string, title?: string, meta?: object }>} [decrypt] - Decrypt entry fields
 * @property {() => { entryCount: number, storageMb: number, maxEntries: number, maxStorageMb: number }} [checkLimits] - Check tier usage limits
 */

/**
 * Shared utilities passed alongside ctx to tool handlers.
 *
 * @typedef {Object} ToolShared
 * @property {() => Promise<void>} ensureIndexed - Ensure auto-reindex has completed
 * @property {boolean} reindexFailed - Whether auto-reindex failed after max attempts
 */
