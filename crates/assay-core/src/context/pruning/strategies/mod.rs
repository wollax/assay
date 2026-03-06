//! Individual pruning strategy implementations.
//!
//! Each strategy is a standalone function with signature:
//! `fn(Vec<ParsedEntry>, PrescriptionTier, &HashSet<usize>) -> StrategyResult`

pub mod metadata_strip;
pub mod progress_collapse;
pub mod stale_reads;
pub mod thinking_blocks;
