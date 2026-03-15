//! Shared serializable types for the Assay development kit.
//!
//! This crate defines the core data structures used across all Assay tools:
//! specs, gates, reviews, workflows, configuration, and supporting types.
//! All types derive `Serialize`/`Deserialize` (serde) and most derive
//! `JsonSchema` (schemars) for automatic schema generation.

#![deny(missing_docs)]

pub mod checkpoint;
pub mod context;
pub mod criterion;
pub mod enforcement;
pub mod evaluator;
pub mod feature_spec;
pub mod gate;
pub mod gate_run;
pub mod gates_spec;
pub mod schema_registry;
pub mod session;
pub mod validation;
pub mod work_session;
pub mod worktree;

pub use checkpoint::{
    AgentState, AgentStatus, ContextHealthSnapshot, TaskState, TaskStatus, TeamCheckpoint,
};
pub use context::{
    BloatCategory, ContextHealth, DiagnosticsReport, SessionEntry, SessionInfo, TokenEstimate,
    UsageData,
};
pub use criterion::{Criterion, CriterionKind};
pub use evaluator::{CriterionOutcome, EvaluatorCriterionResult, EvaluatorOutput, EvaluatorSummary};
pub use enforcement::{Enforcement, EnforcementSummary, GateSection};
pub use feature_spec::FeatureSpec;
pub use gate::{GateKind, GateResult};
pub use gate_run::{CriterionResult, GateRunRecord, GateRunSummary};
pub use gates_spec::{GateCriterion, GatesSpec};
pub use session::{AgentEvaluation, AgentSession, Confidence, EvaluatorRole};
pub use validation::{Diagnostic, DiagnosticSummary, Severity, ValidationResult};
pub use work_session::{AgentInvocation, PhaseTransition, SessionPhase, WorkSession};
pub use worktree::{WorktreeConfig, WorktreeInfo, WorktreeMetadata, WorktreeStatus};

/// Marker badge for directory-based specs in CLI output (e.g., `auth-flow  [srs] 3 criteria`).
///
/// Directory specs store criteria across multiple files rather than in a single TOML.
pub const DIRECTORY_SPEC_INDICATOR: &str = "[srs]";

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A specification that defines what should be built and its acceptance criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Display name for this spec (required, must be unique across all specs).
    pub name: String,

    /// Human-readable description. Defaults to empty string, omitted from
    /// serialized output when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Gate configuration section (enforcement defaults).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateSection>,

    /// Spec names this spec depends on. Used for dependency ordering and cycle detection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,

    /// Acceptance criteria that must be satisfied.
    pub criteria: Vec<Criterion>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "spec",
        generate: || schemars::schema_for!(Spec),
    }
}

/// A quality gate that must pass before work proceeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Gate {
    /// Name of this quality gate.
    pub name: String,
    /// Whether this gate passed.
    pub passed: bool,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "gate",
        generate: || schemars::schema_for!(Gate),
    }
}

/// A review of completed work against a spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Review {
    /// Name of the spec that was reviewed.
    pub spec_name: String,
    /// Whether the review approved the work.
    pub approved: bool,
    /// Reviewer comments. Omitted from serialized output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "review",
        generate: || schemars::schema_for!(Review),
    }
}

/// A workflow combining specs, gates, and reviews into a development pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Workflow {
    /// Name of this workflow.
    pub name: String,
    /// Specs included in this workflow. Omitted from serialized output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specs: Vec<Spec>,
    /// Gates included in this workflow. Omitted from serialized output when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<Gate>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "workflow",
        generate: || schemars::schema_for!(Workflow),
    }
}

/// Top-level configuration for an Assay project.
///
/// Loaded from `.assay/config.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Project name (required, non-empty after trim).
    pub project_name: String,

    /// Directory containing spec files, relative to `.assay/`.
    /// Defaults to `"specs/"`.
    #[serde(default = "default_specs_dir")]
    pub specs_dir: String,

    /// Gate execution configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gates: Option<GatesConfig>,

    /// Guard daemon configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<GuardConfig>,

    /// Worktree management configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<WorktreeConfig>,

    /// Session management configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sessions: Option<SessionsConfig>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "config",
        generate: || schemars::schema_for!(Config),
    }
}

/// Gate execution configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GatesConfig {
    /// Default timeout for gate commands in seconds. Defaults to 300.
    #[serde(default = "default_timeout")]
    pub default_timeout: u64,

    /// Working directory for gate execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    /// Maximum number of run history files to retain per spec.
    /// When set, oldest files beyond this limit are pruned on each save.
    /// A value of `0` means unlimited (no pruning). Defaults to `None` (no pruning).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_history: Option<usize>,

    /// Default model for the evaluator subprocess. Defaults to `"sonnet"`.
    #[serde(default = "default_evaluator_model")]
    pub evaluator_model: String,

    /// Maximum retries for transient evaluator subprocess failures. Defaults to 1.
    #[serde(default = "default_evaluator_retries")]
    pub evaluator_retries: u32,

    /// Evaluator subprocess timeout in seconds. Defaults to 120.
    #[serde(default = "default_evaluator_timeout")]
    pub evaluator_timeout: u64,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "gates-config",
        generate: || schemars::schema_for!(GatesConfig),
    }
}

/// Guard daemon configuration.
///
/// Controls thresholds, polling interval, and circuit breaker behavior
/// for the background context protection daemon.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardConfig {
    /// Soft threshold as percentage of context window (0.0-1.0). Default: 0.6.
    #[serde(default = "default_soft_threshold")]
    pub soft_threshold: f64,

    /// Hard threshold as percentage of context window (0.0-1.0). Default: 0.8.
    #[serde(default = "default_hard_threshold")]
    pub hard_threshold: f64,

    /// Soft threshold as file size in bytes. Optional, whichever fires first wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft_threshold_bytes: Option<u64>,

    /// Hard threshold as file size in bytes. Optional, whichever fires first wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hard_threshold_bytes: Option<u64>,

    /// Polling interval in seconds. Default: 5.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,

    /// Maximum recovery attempts before circuit breaker trips. Default: 3.
    #[serde(default = "default_max_recoveries")]
    pub max_recoveries: u32,

    /// Time window in seconds for counting recoveries. Default: 600 (10 minutes).
    #[serde(default = "default_recovery_window")]
    pub recovery_window_secs: u64,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "guard-config",
        generate: || schemars::schema_for!(GuardConfig),
    }
}

/// Session management configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionsConfig {
    /// Staleness threshold in seconds for recovery sweep.
    /// Sessions in `agent_running` phase older than this are marked abandoned on startup.
    /// Default: 3600 (1 hour).
    #[serde(default = "default_stale_threshold")]
    pub stale_threshold: u64,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "sessions-config",
        generate: || schemars::schema_for!(SessionsConfig),
    }
}

fn default_stale_threshold() -> u64 {
    3600
}

fn default_soft_threshold() -> f64 {
    0.6
}
fn default_hard_threshold() -> f64 {
    0.8
}
fn default_poll_interval() -> u64 {
    5
}
fn default_max_recoveries() -> u32 {
    3
}
fn default_recovery_window() -> u64 {
    600
}

fn default_evaluator_model() -> String {
    "sonnet".to_string()
}

fn default_evaluator_retries() -> u32 {
    1
}

fn default_evaluator_timeout() -> u64 {
    120
}

fn default_specs_dir() -> String {
    "specs/".to_string()
}

fn default_timeout() -> u64 {
    300
}
