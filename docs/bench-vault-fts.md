# `--vault-db` FTS probe

Rust `context-vault bench --vault-db <path>` runs read-only probes. FTS token selection is **not** the fixture `hybrid` path; see `timings.sqlite.fts_user_probe` in JSON output.

## Behaviour

1. **Multi-row scan** — up to **200** indexed rows (`title` + `body`), newest `updated_at`/`created_at` first. Normalized word candidates (4–24 alnum, not all digits, not in a static English **stopword** list). Deduplicated; at most **120** candidates tried.
2. **Smoke test** — each candidate is checked with `SELECT 1 FROM vault_fts WHERE vault_fts MATCH ? LIMIT 1` before it can be used for the timed query.
3. **Fixed fallbacks** — if no candidate matches, try in order: `kind:decision`, `kind:insight`, `decision`, `insight`, `knowledge`, `pattern`, `vault`, `context`, `bench`.
4. **Failure** — if nothing matches, the bench exits with an error (no silent zero-row “bench”).

## Limits

- Does not search unindexed rows (`indexed = 0`).
- Does not implement hybrid semantic / vec lanes (`sqlite-vec` deferred).
- `bench-meta.vault_fts_limits` summarizes the user-vault path when `--vault-db` is set.
