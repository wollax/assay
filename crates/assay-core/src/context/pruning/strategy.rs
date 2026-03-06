//! Strategy enum dispatch and result types.

use std::collections::HashSet;

use assay_types::context::{PrescriptionTier, PruneSample, PruneStrategy};

use super::super::parser::ParsedEntry;

/// Result of applying a single pruning strategy.
pub struct StrategyResult {
    /// The entries after this strategy has been applied.
    pub entries: Vec<ParsedEntry>,
    /// Number of lines removed by this strategy.
    pub lines_removed: usize,
    /// Number of lines modified (content trimmed) by this strategy.
    pub lines_modified: usize,
    /// Bytes saved by this strategy.
    pub bytes_saved: u64,
    /// Number of protected lines skipped.
    pub protected_skipped: usize,
    /// Sample removals for dry-run display (up to 3).
    pub samples: Vec<PruneSample>,
}

/// Apply a pruning strategy to a set of parsed entries.
///
/// Dispatches to the appropriate strategy function. Protected line numbers
/// are skipped by all strategies.
pub fn apply_strategy(
    strategy: &PruneStrategy,
    entries: Vec<ParsedEntry>,
    _tier: PrescriptionTier,
    _protected: &HashSet<usize>,
) -> StrategyResult {
    match strategy {
        PruneStrategy::ProgressCollapse => {
            super::strategies::progress_collapse::progress_collapse(entries, _tier, _protected)
        }
        PruneStrategy::SystemReminderDedup => {
            super::strategies::system_reminder_dedup::system_reminder_dedup(
                entries, _tier, _protected,
            )
        }
        PruneStrategy::MetadataStrip => {
            super::strategies::metadata_strip::metadata_strip(entries, _tier, _protected)
        }
        PruneStrategy::StaleReads => {
            super::strategies::stale_reads::stale_reads(entries, _tier, _protected)
        }
        PruneStrategy::ThinkingBlocks => {
            super::strategies::thinking_blocks::thinking_blocks(entries, _tier, _protected)
        }
        PruneStrategy::ToolOutputTrim => {
            super::strategies::tool_output_trim::tool_output_trim(entries, _tier, _protected)
        }
    }
}
