//! Composable pruning engine for session JSONL files.
//!
//! Provides a pipeline of strategies that reduce session bloat while
//! preserving team coordination messages. Each strategy is a pure function
//! operating on `Vec<ParsedEntry>`.

pub mod protection;
pub mod strategies;
pub mod strategy;

pub use strategy::{StrategyResult, apply_strategy};

use std::collections::HashSet;

use assay_types::context::{PrescriptionTier, PruneStrategy};

use super::parser::ParsedEntry;

/// Execute a pruning pipeline: apply strategies sequentially, collecting results.
///
/// This is a stub that will be fully implemented in plan 04.
pub fn execute_pipeline(
    _entries: Vec<ParsedEntry>,
    _strategies: &[PruneStrategy],
    _tier: PrescriptionTier,
    _protected_lines: &HashSet<usize>,
) -> Vec<StrategyResult> {
    // Full implementation in plan 04.
    Vec::new()
}
