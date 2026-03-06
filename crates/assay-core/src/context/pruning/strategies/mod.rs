//! Individual pruning strategy implementations.
//!
//! Each strategy is a standalone function with signature:
//! `fn(Vec<ParsedEntry>, PrescriptionTier, &HashSet<usize>) -> StrategyResult`

pub mod progress_collapse;
