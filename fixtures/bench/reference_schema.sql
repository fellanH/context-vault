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

-- Situations: entries carry `situations` (array) and `situation_order` in meta (so they live in the
-- markdown frontmatter and survive a reindex). This index is kept by triggers; it is additive and
-- needs no backfill (no earlier entry has situations). The LIKE guards keep every other write free.
CREATE TABLE IF NOT EXISTS vault_situations (
    situation TEXT NOT NULL,
    entry_id  TEXT NOT NULL,
    ord       INTEGER NOT NULL DEFAULT 100,
    updated   TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (situation, entry_id)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_vault_situations_entry ON vault_situations(entry_id);
CREATE INDEX IF NOT EXISTS idx_vault_situations_order ON vault_situations(situation, ord, updated DESC, entry_id);

-- Only a well-formed `situations` array of kebab-case names is indexed; anything else is ignored (the entry
-- stays editable, it just is not in a situation). json_valid/json_type sit in a CASE so they never see bad JSON.
CREATE TRIGGER IF NOT EXISTS vault_situations_ai AFTER INSERT ON vault
WHEN new.meta LIKE '%"situations"%' BEGIN
    INSERT OR REPLACE INTO vault_situations(situation, entry_id, ord, updated)
      SELECT j.value, new.id, COALESCE(CAST(json_extract(new.meta, '$.situation_order') AS INTEGER), 100),
             COALESCE(new.updated_at, new.created_at, '')
        FROM json_each(CASE WHEN json_valid(new.meta) AND json_type(new.meta, '$.situations') = 'array' THEN new.meta ELSE '{}' END, '$.situations') j
       WHERE j.type = 'text' AND length(j.value) BETWEEN 1 AND 32 AND j.value NOT GLOB '*[^a-z0-9-]*' AND j.value NOT GLOB '-*' AND j.value NOT GLOB '*-' AND j.value NOT GLOB '*--*';
END;
CREATE TRIGGER IF NOT EXISTS vault_situations_au AFTER UPDATE OF meta, updated_at ON vault
WHEN old.meta LIKE '%"situations"%' OR new.meta LIKE '%"situations"%' BEGIN
    DELETE FROM vault_situations WHERE entry_id = old.id;
    INSERT OR REPLACE INTO vault_situations(situation, entry_id, ord, updated)
      SELECT j.value, new.id, COALESCE(CAST(json_extract(new.meta, '$.situation_order') AS INTEGER), 100),
             COALESCE(new.updated_at, new.created_at, '')
        FROM json_each(CASE WHEN json_valid(new.meta) AND json_type(new.meta, '$.situations') = 'array' THEN new.meta ELSE '{}' END, '$.situations') j
       WHERE j.type = 'text' AND length(j.value) BETWEEN 1 AND 32 AND j.value NOT GLOB '*[^a-z0-9-]*' AND j.value NOT GLOB '-*' AND j.value NOT GLOB '*-' AND j.value NOT GLOB '*--*';
END;
CREATE TRIGGER IF NOT EXISTS vault_situations_ad AFTER DELETE ON vault
WHEN old.meta LIKE '%"situations"%' BEGIN
    DELETE FROM vault_situations WHERE entry_id = old.id;
END;

-- The cached vault map (one row): regenerated when older than 24 h or the entry count moved.
CREATE TABLE IF NOT EXISTS vault_map (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    map_json     TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    entries_at   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_vault_kind_cat_created ON vault(kind, category, created_at DESC);

PRAGMA user_version = 20;
