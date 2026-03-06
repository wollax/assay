//! Individual pruning strategy implementations.
//!
//! Each strategy is a standalone function with signature:
//! `fn(Vec<ParsedEntry>, PrescriptionTier, &HashSet<usize>) -> StrategyResult`
//!
//! Strategy modules will be added in subsequent plans:
//! - progress_collapse
//! - system_reminder_dedup
//! - metadata_strip
//! - stale_reads
//! - thinking_blocks
//! - tool_output_trim
