//! Shared serializable types for the Assay development kit.
//!
//! This crate defines the core data structures used across all Assay tools:
//! specs, gates, reviews, workflows, configuration, and supporting types.
//! All types derive `Serialize`/`Deserialize` (serde) and most derive
//! `JsonSchema` (schemars) for automatic schema generation.

#![deny(missing_docs)]

pub mod agent_event;
pub mod checkpoint;
pub mod context;
pub mod coverage;
pub mod criteria_library;
pub mod criterion;
pub mod enforcement;
pub mod evaluator;
pub mod evidence;
pub mod feature_spec;
pub mod gate;
pub mod gate_run;
pub mod gates_spec;
pub mod harness;
pub mod manifest;
pub mod merge;
pub mod milestone;
pub mod precondition;
pub mod provider;
pub mod resolved_gate;
pub mod review;
pub mod schema_registry;
pub mod session;
pub mod signal;
pub mod state_backend;
pub mod validation;
pub mod wizard_input;
pub mod work_session;
pub mod worktree;

#[cfg(feature = "orchestrate")]
pub mod orchestrate;

pub use agent_event::AgentEvent;
pub use checkpoint::{
    AgentState, AgentStatus, ContextHealthSnapshot, TaskState, TaskStatus, TeamCheckpoint,
};
pub use context::{
    BloatCategory, ContextHealth, DiagnosticsReport, SessionEntry, SessionInfo, TokenEstimate,
    UsageData,
};
pub use coverage::CoverageReport;
pub use criteria_library::CriteriaLibrary;
pub use criterion::{Criterion, CriterionKind};
pub use enforcement::{Enforcement, EnforcementSummary, GateSection};
pub use evaluator::{
    CriterionOutcome, EvaluatorCriterionResult, EvaluatorOutput, EvaluatorSummary,
};
pub use evidence::FormattedEvidence;
pub use feature_spec::{FeatureSpec, SpecStatus};
pub use gate::{GateKind, GateResult};
pub use gate_run::{
    CriterionResult, DiffTruncation, GateEvalOutcome, GateRunRecord, GateRunSummary,
};
pub use gates_spec::{GateCriterion, GateSpecStatus, GatesSpec};
pub use harness::{
    HarnessProfile, HookContract, HookEvent, PromptLayer, PromptLayerKind, ScopeViolation,
    ScopeViolationType, SettingsOverride,
};
pub use manifest::{ManifestSession, RunManifest};
pub use merge::{
    ChangeType, ConflictMarker, ConflictScan, ConflictType, FileChange, MarkerType, MergeCheck,
    MergeConflict, MergeExecuteResult, MergeProposal, MergeProposeConfig,
};
pub use milestone::{ChunkRef, Milestone, MilestoneStatus};
pub use precondition::{CommandStatus, PreconditionStatus, RequireStatus, SpecPreconditions};
pub use provider::{HarnessError, HarnessProvider, NullProvider};
pub use resolved_gate::{CriterionSource, ResolvedCriterion, ResolvedGate};
pub use review::{
    FailedCriterionSummary, GateDiagnostic, ReviewCheck, ReviewCheckKind, ReviewReport,
};
pub use session::{AgentEvaluation, Confidence, EvaluatorRole, GateEvalContext};
pub use signal::{
    AssayServerState, GateSummary, PeerInfo, PeerUpdate, PollSignalsResult, RunSummary,
    SignalRequest,
};
pub use state_backend::StateBackendConfig;
pub use validation::{Diagnostic, DiagnosticSummary, Severity, ValidationResult};
pub use wizard_input::{
    CriteriaWizardInput, CriteriaWizardOutput, CriterionInput, GateWizardInput, GateWizardOutput,
};
pub use work_session::{AgentInvocation, PhaseTransition, SessionPhase, WorkSession};
pub use worktree::{WorktreeConfig, WorktreeInfo, WorktreeMetadata, WorktreeStatus};

#[cfg(feature = "orchestrate")]
pub use orchestrate::{
    ConflictAction, ConflictFileContent, ConflictResolution, ConflictResolutionConfig,
    FailurePolicy, GossipConfig, GossipStatus, KnowledgeEntry, KnowledgeManifest, MergePlan,
    MergePlanEntry, MergeReport, MergeSessionResult, MergeSessionStatus, MergeStrategy, MeshConfig,
    MeshMemberState, MeshMemberStatus, MeshStatus, OrchestratorMode, OrchestratorPhase,
    OrchestratorStatus, SessionRunState, SessionStatus,
};

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

    /// AI provider configuration (Anthropic, OpenAI, Ollama).
    /// Omit to use the default provider (Anthropic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderConfig>,

    /// Workflow behavior configuration (solo developer settings).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowConfig>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "config",
        generate: || schemars::schema_for!(Config),
    }
}

/// AI provider selection.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Anthropic (Claude).
    #[default]
    Anthropic,
    /// OpenAI (GPT).
    OpenAi,
    /// Ollama (local models).
    Ollama,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "provider_kind",
        generate: || schemars::schema_for!(ProviderKind),
    }
}

/// AI provider and model configuration.
///
/// All fields are optional and omitted from serialized output when absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    /// Which AI provider to use. Defaults to Anthropic.
    #[serde(default)]
    pub provider: ProviderKind,

    /// Model to use for planning tasks (e.g. "claude-opus-4-5").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planning_model: Option<String>,

    /// Model to use for execution tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_model: Option<String>,

    /// Model to use for review tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_model: Option<String>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "provider_config",
        generate: || schemars::schema_for!(ProviderConfig),
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

    /// How to handle `AgentReport` criteria: `"auto"` (evaluator subprocess)
    /// or `"manual"` (gate_report flow). Defaults to `"auto"`.
    #[serde(default = "default_agent_eval_mode")]
    pub agent_eval_mode: String,
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
    /// Sessions in the `AgentRunning` phase older than this are marked abandoned on startup.
    /// Default: 3600 (1 hour). Must be greater than zero.
    #[serde(default = "default_stale_threshold", alias = "stale_threshold")]
    pub stale_threshold_secs: u64,

    /// Maximum number of sessions to retain. Oldest sessions beyond this limit
    /// are evicted (unless linked to an InProgress milestone).
    /// Default: 100. Set to 0 to disable count-based eviction.
    #[serde(default = "default_max_session_count")]
    pub max_count: usize,

    /// Maximum age of sessions in days. Sessions older than this are evicted
    /// (unless linked to an InProgress milestone).
    /// Default: 90. Set to 0 to disable age-based eviction.
    #[serde(default = "default_max_session_age_days")]
    pub max_age_days: u64,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            stale_threshold_secs: default_stale_threshold(),
            max_count: default_max_session_count(),
            max_age_days: default_max_session_age_days(),
        }
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "sessions-config",
        generate: || schemars::schema_for!(SessionsConfig),
    }
}

/// Branch isolation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AutoIsolate {
    /// Always create a worktree/branch before starting work.
    Always,
    /// Never auto-isolate; the user manages branches.
    Never,
    /// Prompt if on a protected branch; proceed silently on a feature branch.
    #[default]
    Ask,
}

impl std::fmt::Display for AutoIsolate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Always => write!(f, "always"),
            Self::Never => write!(f, "never"),
            Self::Ask => write!(f, "ask"),
        }
    }
}

/// Workflow behavior configuration for the solo developer loop.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowConfig {
    /// Branch isolation strategy. Default: `ask`.
    #[serde(default)]
    pub auto_isolate: AutoIsolate,

    /// Protected branch names. When set, overrides the built-in list
    /// (`["main", "master", "develop"]` + dynamic detection).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected_branches: Option<Vec<String>>,

    /// Whether UAT is required after gate pass. Default: `false`.
    #[serde(default)]
    pub uat_enabled: bool,

    /// Whether spec status must be >= `approved` before running gates. Default: `false`.
    #[serde(default)]
    pub strict_status: bool,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "workflow-config",
        generate: || schemars::schema_for!(WorkflowConfig),
    }
}

fn default_stale_threshold() -> u64 {
    3600
}

fn default_max_session_count() -> usize {
    100
}

fn default_max_session_age_days() -> u64 {
    90
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

fn default_agent_eval_mode() -> String {
    "auto".to_string()
}

fn default_specs_dir() -> String {
    "specs/".to_string()
}

fn default_timeout() -> u64 {
    300
}
