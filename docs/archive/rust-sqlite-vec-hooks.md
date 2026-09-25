> **Archived 2026-07-03.** Resolved — `sqlite-vec` does load via `rusqlite`'s bundled feature;
> it's wired into `crates/cv-backends/src/sqlite.rs` and `sqlite_ops.rs`. Kept for historical
> context on the original open question.

# sqlite-vec hooks

**Status:** Resolved — see archive note above.

The key open question: does `sqlite-vec` load correctly when SQLite is bundled via `rusqlite`
(feature `bundled`)? Verify before implementing the `SqliteBackend` vector lane.

## Plan (when ready)

1. Load `sqlite-vec` extension via `rusqlite`'s `load_extension` once the bundled-SQLite
   compatibility is confirmed.
2. Mirror `vault_vec` / `vault_ctx_vec` DDL from `packages/core/src/db.ts` — schema must be
   identical so existing databases load without migration.
3. Bench output adds a separate `vec_lane` object so FTS timings are never conflated with
   vector timings (`docs/bench-vault-fts.md` covers the FTS probe behaviour).

**Do not implement until** `SqliteBackend` FTS lane has ranking parity with the TypeScript core
on bench mode B (file-backed SQLite + honest filters).
