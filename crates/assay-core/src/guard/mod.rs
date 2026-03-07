//! Guard daemon: background context protection with threshold-based pruning.

pub mod circuit_breaker;
pub mod config;
pub mod pid;
pub mod thresholds;
pub mod watcher;
