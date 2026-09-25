pub mod taxonomy;
pub mod schema_types;
pub mod schema_check;
mod schema_personal;
mod kind_suggest;
#[cfg(test)]
mod schema_test_support;
pub mod reorg;
pub mod relocate;
pub mod frontmatter;
pub mod fm_parse;
pub mod md_extract;
pub mod formatters;
pub mod files;
pub mod entry_file;
pub mod capture;

pub use taxonomy::{FieldSpec, FieldType, KindRule, VaultPolicy, VaultTaxonomy};
pub use schema_check::check_save;
pub use reorg::{apply_reorg, plan_reorg, ReorgSummary};
pub use relocate::{plan_move, MoveSummary};
pub use frontmatter::format_frontmatter;
pub use fm_parse::{parse_frontmatter, extract_custom_meta};
pub use md_extract::parse_entry_from_markdown;
pub use formatters::format_body;
pub use files::{slugify, safe_join, walk_dir, WalkResult};
pub use entry_file::write_entry_file;
pub use capture::{write_entry, WriteResult};
