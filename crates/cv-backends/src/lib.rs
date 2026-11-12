pub mod embedder;
pub(crate) mod filters;
pub mod queries;
pub mod reindex;
mod reindex_scan;
pub mod row_map;
pub mod sqlite;
mod situation_cursor;
pub mod situations;
#[cfg(test)]
mod situations_tests;
mod sqlite_backfill;
mod sqlite_situations;
pub mod sqlite_ops;
mod sqlite_write;
pub mod vault_map;
mod vault_map_areas;
mod vault_map_query;
#[cfg(test)]
mod vault_map_tests;

pub use embedder::{FastembedEmbedder, NoOpEmbedder};
pub use sqlite::SqliteBackend;
