/// Error types for Assay operations.
pub mod error;

pub use error::{AssayError, EvaluatorError, Result};

/// Spec authoring and validation.
pub mod spec;

/// Quality gate evaluation.
pub mod gate;

/// Work review against specs.
pub mod review;

/// Workflow orchestration.
pub mod workflow;

/// Configuration loading and validation.
pub mod config;

/// Run manifest loading and validation.
pub mod manifest;

/// Run history persistence.
pub mod history;

/// Project initialization.
pub mod init;

/// Team state checkpointing: extraction, persistence, and team config discovery.
pub mod checkpoint;

/// Claude Code session parsing, discovery, and token diagnostics.
pub mod context;

/// Guard daemon: background context protection with threshold-based pruning.
pub mod guard;

/// Git worktree lifecycle management.
pub mod worktree;

/// Work session lifecycle management.
pub mod work_session;

/// Evaluator subprocess: spawn, parse, and map Claude Code evaluator results.
pub mod evaluator;

/// Merge check: conflict detection between git refs with zero side effects.
pub mod merge;

/// End-to-end pipeline orchestrator: manifest → worktree → harness → agent → gate → merge.
pub mod pipeline;

/// Milestone I/O: scan, load, and save milestones under `.assay/milestones/`.
pub mod milestone;

/// Session dependency orchestration: DAG construction and validation.
///
/// Gated behind the `orchestrate` Cargo feature.
#[cfg(feature = "orchestrate")]
pub mod orchestrate;
