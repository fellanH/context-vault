pub use cv_backends as backends;
pub mod bench_harness;
// The engine contract lives in the `cv-engine` crate; re-export it as
// `context_vault_core::engine` so existing `crate::engine::*` /
// `context_vault_core::engine::*` paths keep resolving after the split.
pub use cv_engine as engine;
pub use cv_vault as vault;
pub use cv_schema as schema;
pub use cv_schema::indexing;
pub use cv_lifecycle as lifecycle;
pub use cv_ingest::{ingest, watch};
pub mod mcp;
pub mod paths;
