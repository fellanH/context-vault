-- Bench reference schema (stock SQLite; no sqlite-vec).
-- Frozen snapshot aligned with packages/core/src/db.ts SCHEMA_DDL relational + FTS5
-- surface. Omits vec0 tables (vault_vec, vault_ctx_vec) so `sqlite3` / bundled rusqlite
-- can materialize the DB without native extensions.
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS vault (
    id              TEXT PRIMARY KEY,
    kind            TEXT NOT NULL,
    category        TEXT NOT NULL DEFAULT 'knowledge',
    title           TEXT,
    body            TEXT NOT NULL,
    meta            TEXT,
    tags            TEXT,
    source          TEXT,
    file_path       TEXT UNIQUE,
    identity_key    TEXT,
    expires_at      TEXT,
    superseded_by   TEXT,
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT,
    hit_count       INTEGER DEFAULT 0,
    last_accessed_at TEXT,
    source_files    TEXT,
    tier            TEXT DEFAULT 'working' CHECK(tier IN ('ephemeral', 'working', 'durable')),
    related_to      TEXT,
    indexed         INTEGER DEFAULT 1,
    recall_count    INTEGER DEFAULT 0,
    recall_sessions INTEGER DEFAULT 0,
    last_recalled_at TEXT,
    heat_tier       TEXT CHECK(heat_tier IN ('hot', 'warm', 'cold')),
    summary_condensed TEXT,
    summary_keypoint  TEXT
);

CREATE INDEX IF NOT EXISTS idx_vault_kind ON vault(kind);
CREATE INDEX IF NOT EXISTS idx_vault_category ON vault(category);
CREATE INDEX IF NOT EXISTS idx_vault_category_created ON vault(category, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_vault_updated ON vault(updated_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_vault_identity ON vault(kind, identity_key) WHERE identity_key IS NOT NULL AND category = 'entity';
CREATE INDEX IF NOT EXISTS idx_vault_superseded ON vault(superseded_by) WHERE superseded_by IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_vault_tier ON vault(tier);
CREATE INDEX IF NOT EXISTS idx_vault_indexed ON vault(indexed);
CREATE INDEX IF NOT EXISTS idx_vault_heat_tier ON vault(heat_tier) WHERE heat_tier IS NOT NULL;

CREATE VIRTUAL TABLE IF NOT EXISTS vault_fts USING fts5(
    title, body, tags, kind,
    content='vault', content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS vault_ai AFTER INSERT ON vault WHEN new.indexed = 1 BEGIN
    INSERT INTO vault_fts(rowid, title, body, tags, kind)
      VALUES (new.rowid, new.title, new.body, new.tags, new.kind);
END;
CREATE TRIGGER IF NOT EXISTS vault_ad AFTER DELETE ON vault WHEN old.indexed = 1 BEGIN
    INSERT INTO vault_fts(vault_fts, rowid, title, body, tags, kind)
      VALUES ('delete', old.rowid, old.title, old.body, old.tags, old.kind);
END;
CREATE TRIGGER IF NOT EXISTS vault_au AFTER UPDATE ON vault WHEN old.indexed = 1 OR new.indexed = 1 BEGIN
    INSERT INTO vault_fts(vault_fts, rowid, title, body, tags, kind)
      VALUES ('delete', old.rowid, old.title, old.body, old.tags, old.kind);
    INSERT INTO vault_fts(rowid, title, body, tags, kind)
      SELECT new.rowid, new.title, new.body, new.tags, new.kind WHERE new.indexed = 1;
END;

CREATE TABLE IF NOT EXISTS co_retrievals (
    entry_a TEXT NOT NULL,
    entry_b TEXT NOT NULL,
    count INTEGER DEFAULT 1,
    last_at TEXT NOT NULL,
    PRIMARY KEY (entry_a, entry_b)
);

CREATE TABLE IF NOT EXISTS access_log (
    id          INTEGER PRIMARY KEY,
    entry_id    TEXT NOT NULL,
    query       TEXT,
    session_id  TEXT,
    session_goal TEXT,
    accessed_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_access_log_entry_at ON access_log(entry_id, accessed_at);
CREATE INDEX IF NOT EXISTS idx_access_log_goal ON access_log(session_goal) WHERE session_goal IS NOT NULL;

-- Incremental reindex watermark (reindex_started_at) and similar engine state.
CREATE TABLE IF NOT EXISTS index_state (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

PRAGMA user_version = 20;
