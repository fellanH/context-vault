//! Lifecycle module: summarization, compaction, archival, consolidation, and tiering.
//!
//! ## Modules
//! - `summarize`: Two-tier text summaries (condensed + keypoint)
//! - `compact`: Gzip cold entry bodies to _archive/ and restore
//! - `archive`: Age-based entry archival to _archive/ subdirectories
//! - `archive_filter`: Archive by kind, tier and created range, with a manifest and its restore (OPS-406)
//! - `assemble`: Role-based context assembly with token budgeting
//! - `consolidation`: Deduplication via similarity and decay scoring
//! - `tier_analysis`: Hot/warm/cold tiering via access logs

pub mod summarize;
pub mod compact;
pub mod archive;
pub mod archive_filter;
pub mod archive_manifest;
#[cfg(test)]
mod archive_filter_tests;
pub mod assemble;
pub mod consolidation;
pub mod tier_analysis;

// Re-export primary entry points
pub use summarize::{generate_summary_tiers, SummaryTiers};
pub use compact::{compact, restore_compacted_body, find_compact_candidates, summarize_body, CompactResult, CompactOptions};
pub use archive::{
    find_archive_candidates, archive_entries, restore_entry, count_archived_entries,
    ArchiveCandidate, ArchiveResult, RestoreResult, ArchivedEntry,
};
pub use assemble::{assemble_context, Role, AssembleResult, AssembleMetadata};
pub use consolidation::{
    find_duplicates, merge_entries, compute_decay_scores,
    DuplicateGroup, MergeResult, DecayScore, FindDuplicatesOptions,
};
pub use tier_analysis::{
    run_tier_analysis, TierAnalysisReport, HotEntry, ColdEntry, CoAccessBundle,
    TierAnalysisOptions,
};
