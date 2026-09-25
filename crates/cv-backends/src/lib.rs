pub mod embedder;
pub(crate) mod filters;
pub mod queries;
pub mod reindex;
mod reindex_scan;
pub mod row_map;
pub mod sqlite;
pub mod sqlite_ops;

pub use embedder::{FastembedEmbedder, NoOpEmbedder};
pub use sqlite::SqliteBackend;
