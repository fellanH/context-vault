# cv-index (staged v5)

A derived Tantivy index beside the released v4 engine. `cv-index-check` is a maintainer validation binary; the installed CLI/MCP has not switched to it.

```sh
cargo run -p cv-index --bin cv-index-check -- --root /path/to/vault --index /separate/path/to/index index --full
cargo run -p cv-index --bin cv-index-check -- --root /path/to/vault --index /separate/path/to/index search '"webhook retry"' --meta '{"currency":"USD"}'
```

Implemented: Markdown and JSONL/zstd source locations, BM25 text/phrases, exact kind/stream/tag/scalar metadata filters, created-date bounds, source-bound atomic checkpoints, incremental updates/deletions, non-blocking writer contention and sealed-segment offsets. Metadata string/number types are distinct. A malformed record or duplicated ID aborts rather than silently discarding content. Source files remain authoritative.

The catch-up budget is soft: commit/merge and traversal prefix can take longer. It has not established the production 200 ms target at scale. Source IDs must be persisted before removing an old database that holds legacy identities. Default supersession handling, full production surface, replay/migration gates, ranking evaluation and removals remain work for v5. This crate does not establish hosted engine convergence or hundreds-of-GB performance.
