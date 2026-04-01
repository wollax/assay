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

/// Gate-gated PR creation workflow.
pub mod pr;

/// Guided authoring wizard: pure functions for creating milestones and specs.
pub mod wizard;

/// Centralized tracing subscriber initialization for all binaries.
pub mod telemetry;

/// Session dependency orchestration: DAG construction and validation.
///
/// Gated behind the `orchestrate` Cargo feature.
#[cfg(feature = "orchestrate")]
pub mod orchestrate;

/// StateBackend trait, CapabilitySet flags struct, and LocalFsBackend skeleton.
///
/// Gated behind the `orchestrate` Cargo feature.
#[cfg(feature = "orchestrate")]
pub mod state_backend;

/// Manifest generation from milestone chunks or all specs.
///
/// Produces a [`RunManifest`] TOML from a milestone's chunk list or
/// from all specs. Gated behind the `orchestrate` Cargo feature
/// because the output uses [`OrchestratorMode`](assay_types::OrchestratorMode).
#[cfg(feature = "orchestrate")]
pub mod manifest_gen;
#[cfg(feature = "orchestrate")]
pub use state_backend::{CapabilitySet, LocalFsBackend, NoopBackend, StateBackend};
