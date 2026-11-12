//! v5 core (spec: specs/v5-storage-and-search.md): the vault registry, the record type
//! shared by both vault types, and their on-disk formats. Knowledge vaults keep v4's
//! one-markdown-file-per-record layout; data vaults are append-only JSONL segments,
//! sealed to zstd once their day has passed. No index lives here (that is cv-index).

pub mod markdown;
pub mod record;
pub mod registry;
pub mod segment_read;
pub mod segment_append;
pub mod segment_write;

pub use record::Record;
pub use registry::{Registry, VaultEntry, VaultType};
pub use segment_read::{read_at, read_segment, segment_files, Loc};
pub use segment_append::{append, append_batch};
pub use segment_write::seal_before;

#[cfg(test)]
mod registry_tests;
#[cfg(test)]
mod segment_tests;
#[cfg(test)]
mod markdown_tests;
