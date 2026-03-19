//! MCP server implementation with spec, gate, context, worktree, session, and milestone tools.
//!
//! Provides the [`AssayServer`] which exposes tools over MCP:
//! - `spec_list` — discover available specs
//! - `spec_get` — read a full spec definition
//! - `spec_validate` — statically validate a spec without running it
//! - `gate_run` — evaluate quality gate criteria (auto-creates sessions for agent criteria)
//! - `gate_evaluate` — evaluate all criteria via headless Claude Code subprocess
//! - `gate_report` — submit agent evaluation for a criterion in an active session
//! - `gate_finalize` — finalize a session, persisting all evaluations as a GateRunRecord
//! - `gate_history` — query past gate run results for a spec
//! - `context_diagnose` — diagnose token usage and bloat in a Claude Code session
//! - `estimate_tokens` — estimate current token usage and context window health
//! - `worktree_create` — create an isolated git worktree for a spec
//! - `worktree_list` — list all active assay-managed worktrees
//! - `worktree_status` — check worktree status (branch, dirty, ahead/behind)
//! - `worktree_cleanup` — remove a worktree and its branch
//! - `merge_check` — check for merge conflicts between two refs (read-only, zero side effects)
//! - `session_create` — create a new work session for a spec
//! - `session_get` — retrieve full session details by ID
//! - `session_update` — transition session phase and link gate runs
//! - `session_list` — list sessions with optional spec_name and status filters
//! - `milestone_list` — list all milestones in the current project
//! - `milestone_get` — get full details of a milestone by slug
//!
//! All domain errors are returned as `CallToolResult` with `isError: true`
//! so that agents can see and self-correct. Protocol errors (`McpError`)
//! are reserved for infrastructure failures.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
    transport::io::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use assay_core::milestone::{milestone_load, milestone_scan};
use assay_core::spec::SpecEntry;
use assay_types::work_session::SessionPhase;
use assay_types::{
    AgentEvaluation, Confidence, Config, CriterionKind, EvaluatorRole, GateEvalContext,
    OrchestratorMode,
};

// ── Parameter structs ────────────────────────────────────────────────

/// Parameters for the `spec_get` tool.
#[derive(Deserialize, JsonSchema)]
pub struct SpecGetParams {
    /// The spec to retrieve.
    #[schemars(description = "Spec name (filename without .toml extension, e.g. 'auth-flow')")]
    pub name: String,

    #[schemars(
        description = "Include resolved configuration (effective timeouts, working_dir validation)"
    )]
    #[serde(default)]
    pub resolve: bool,
}

/// Parameters for the `spec_validate` tool.
#[derive(Deserialize, JsonSchema)]
pub struct SpecValidateParams {
    /// The spec to validate.
    #[schemars(
        description = "Spec name to validate (filename without .toml extension, e.g. 'auth-flow')"
    )]
    pub name: String,

    /// Whether to validate command existence on PATH.
    #[schemars(
        description = "Validate that command binaries in criteria exist on PATH (default: false). \
            Enable for deployment readiness checks. Command-not-found is reported as a warning, \
            not an error, since the execution environment may differ."
    )]
    #[serde(default)]
    pub check_commands: bool,
}

/// Parameters for the `gate_run` tool.
#[derive(Deserialize, JsonSchema)]
pub struct GateRunParams {
    /// The spec whose criteria to evaluate.
    #[schemars(
        description = "Spec name to evaluate gates for (filename without .toml extension, e.g. 'auth-flow')"
    )]
    pub name: String,

    /// Whether to include full evidence in the response.
    #[schemars(
        description = "Include full stdout/stderr evidence per criterion (default: false). \
            When false, returns summary only with pass/fail, exit code, and duration. \
            When true, adds stdout and stderr fields to each criterion result."
    )]
    #[serde(default)]
    pub include_evidence: bool,

    /// Maximum seconds to wait for gate evaluation.
    #[schemars(
        description = "Maximum seconds to wait for gate evaluation (default: 300). \
            Returns a timeout error if exceeded."
    )]
    #[serde(default)]
    pub timeout: Option<u64>,
}

/// Parameters for the `gate_report` tool.
#[derive(Deserialize, JsonSchema)]
pub struct GateReportParams {
    /// In-memory GateEvalContext ID returned by `gate_run` when the spec contains agent criteria.
    /// This is distinct from the persisted WorkSession ID created by `session_create`.
    #[schemars(
        description = "In-memory GateEvalContext ID returned by gate_run when the spec has agent \
            criteria. Distinct from the persisted WorkSession ID created by session_create."
    )]
    pub session_id: String,

    /// Name of the criterion being evaluated (must match a criterion in the spec).
    #[schemars(
        description = "Name of the criterion being evaluated (must match a criterion in the spec)"
    )]
    pub criterion_name: String,

    /// Whether the criterion passed.
    #[schemars(description = "Whether the criterion passed")]
    pub passed: bool,

    /// What the agent observed (concrete facts).
    #[schemars(description = "What the agent observed (concrete facts)")]
    pub evidence: String,

    /// Why those facts lead to pass/fail.
    #[schemars(description = "Why those facts lead to pass/fail")]
    pub reasoning: String,

    /// Confidence in the evaluation (high, medium, low).
    #[schemars(description = "Confidence in the evaluation (high, medium, low)")]
    #[serde(default)]
    pub confidence: Option<Confidence>,

    /// Role of the evaluator (self, independent, human).
    #[schemars(description = "Role of the evaluator (self, independent, human)")]
    pub evaluator_role: EvaluatorRole,
}

/// Parameters for the `gate_finalize` tool.
#[derive(Deserialize, JsonSchema)]
pub struct GateFinalizeParams {
    /// Session ID to finalize.
    #[schemars(description = "Session ID to finalize")]
    pub session_id: String,
}

/// Parameters for the `gate_history` tool.
#[derive(Deserialize, JsonSchema)]
pub struct GateHistoryParams {
    /// Spec name to query history for.
    #[schemars(description = "Spec name to query history for (filename without .toml extension)")]
    pub name: String,

    /// Optional run ID to retrieve a specific record. When omitted, returns a list of recent runs.
    #[schemars(
        description = "Specific run ID to retrieve full details for. Omit to list recent runs."
    )]
    #[serde(default)]
    pub run_id: Option<String>,

    /// Maximum number of runs to return in list mode (default: 10, max: 50).
    #[schemars(
        description = "Maximum number of runs to return (default: 10, max: 50, ignored when run_id is set)"
    )]
    #[serde(default)]
    pub limit: Option<usize>,

    /// Filter by outcome: "passed", "failed", or "any" (default: "any").
    /// A run is "failed" when any required criterion failed (required_failed > 0).
    #[schemars(
        description = "Filter runs by outcome: 'passed' (no required failures), 'failed' (has required failures), or 'any' (default: 'any')"
    )]
    #[serde(default)]
    pub outcome: Option<String>,
}

/// Parameters for the `context_diagnose` tool.
#[derive(Deserialize, JsonSchema)]
pub struct ContextDiagnoseParams {
    /// Session ID to diagnose. Defaults to the most recent session.
    #[schemars(
        description = "Session UUID (e.g. '3201041c-df85-4c91-a485-7b8c189f7636'). Omit for most recent session."
    )]
    pub session_id: Option<String>,
}

/// Parameters for the `estimate_tokens` tool.
#[derive(Deserialize, JsonSchema)]
pub struct EstimateTokensParams {
    /// Session ID to estimate. Defaults to the most recent session.
    #[schemars(description = "Session UUID. Omit for most recent session.")]
    pub session_id: Option<String>,
}

/// Parameters for the `worktree_create` tool.
#[derive(Deserialize, JsonSchema)]
pub struct WorktreeCreateParams {
    /// Spec name (slug) to create a worktree for.
    #[schemars(description = "Spec name (slug, e.g. 'auth-flow')")]
    pub name: String,

    /// Base branch override. Defaults to auto-detected default branch.
    #[schemars(description = "Base branch to create worktree from (default: auto-detected)")]
    #[serde(default)]
    pub base: Option<String>,

    /// Worktree base directory override.
    #[schemars(description = "Override worktree base directory")]
    #[serde(default)]
    pub worktree_dir: Option<String>,
}

/// Parameters for the `worktree_list` tool.
#[derive(Deserialize, JsonSchema)]
pub struct WorktreeListParams {
    /// Worktree base directory override.
    /// Currently unused — list discovers worktrees from git, not the filesystem.
    /// Retained for API consistency with other worktree tools.
    #[schemars(
        description = "Override worktree base directory. Currently unused — list discovers worktrees from git, not the filesystem. Reserved for future use."
    )]
    #[serde(default)]
    #[allow(dead_code)]
    pub worktree_dir: Option<String>,
}

/// Parameters for the `worktree_status` tool.
#[derive(Deserialize, JsonSchema)]
pub struct WorktreeStatusParams {
    /// Spec name (slug) to check status for.
    #[schemars(description = "Spec name (slug, e.g. 'auth-flow')")]
    pub name: String,

    /// Worktree base directory override.
    #[schemars(description = "Override worktree base directory")]
    #[serde(default)]
    pub worktree_dir: Option<String>,

    /// Whether to fetch the base branch from the remote before computing status.
    /// Defaults to false.
    #[schemars(
        description = "Fetch base branch from remote before computing ahead/behind (default: false)"
    )]
    #[serde(default)]
    pub fetch: Option<bool>,
}

/// Parameters for the `worktree_cleanup` tool.
#[derive(Deserialize, JsonSchema)]
pub struct WorktreeCleanupParams {
    /// Spec name (slug) to clean up.
    #[schemars(description = "Spec name (slug, e.g. 'auth-flow')")]
    pub name: String,

    /// Force cleanup of dirty worktrees. Defaults to true for MCP (non-interactive).
    #[schemars(
        description = "Force cleanup even if worktree has uncommitted changes (default: true for MCP)"
    )]
    #[serde(default)]
    pub force: Option<bool>,

    /// Worktree base directory override.
    #[schemars(description = "Override worktree base directory")]
    #[serde(default)]
    pub worktree_dir: Option<String>,
}

/// Parameters for the `merge_check` tool.
#[derive(Deserialize, JsonSchema)]
pub struct MergeCheckParams {
    /// The base ref to merge into (e.g., 'main', 'origin/main', a SHA, or relative ref like 'HEAD~3').
    #[schemars(
        description = "Base ref (the branch being merged into). Accepts branches, tags, SHAs, or relative refs."
    )]
    pub base: String,

    /// The head ref to merge from (e.g., 'feature/auth', a SHA, or relative ref).
    #[schemars(
        description = "Head ref (the branch being merged). Accepts branches, tags, SHAs, or relative refs."
    )]
    pub head: String,

    /// Maximum number of conflicts to return. Default: 20. Excess conflicts are dropped; the `truncated` field indicates when the list was cut.
    #[schemars(
        description = "Maximum conflicts to return in detail (default: 20). The `truncated` field is set when excess conflicts are dropped."
    )]
    #[serde(default)]
    pub max_conflicts: Option<u32>,
}

/// Parameters for the `session_create` tool.
#[derive(Deserialize, JsonSchema)]
pub struct SessionCreateParams {
    /// Spec name this session is for.
    #[schemars(description = "Spec name (slug, e.g. 'auth-flow')")]
    pub spec_name: String,

    /// Path to the worktree associated with this session.
    #[schemars(description = "Absolute path to the worktree directory")]
    pub worktree_path: PathBuf,

    /// Command used to invoke the agent.
    #[schemars(description = "Agent invocation command (e.g. 'claude --model sonnet')")]
    pub agent_command: String,

    /// Model used by the agent, if known.
    #[schemars(description = "Model used by the agent (e.g. 'claude-sonnet-4')")]
    #[serde(default)]
    pub agent_model: Option<String>,
}

/// Parameters for the `session_get` tool.
#[derive(Deserialize, JsonSchema)]
pub struct SessionGetParams {
    /// Session ID to retrieve.
    #[schemars(description = "Session ID (ULID string) to retrieve")]
    pub session_id: String,
}

/// Parameters for the `session_update` tool.
#[derive(Deserialize, JsonSchema)]
pub struct SessionUpdateParams {
    /// Session ID to update.
    #[schemars(description = "Session ID (ULID string) to update")]
    pub session_id: String,

    /// Target phase to transition to.
    #[schemars(
        description = "Target phase: 'agent_running', 'gate_evaluated', 'completed', or 'abandoned'"
    )]
    pub phase: SessionPhase,

    /// What triggered the transition.
    #[schemars(
        description = "What triggered the transition (e.g. 'agent_started', 'gate_passed', 'user_abandoned')"
    )]
    pub trigger: String,

    /// Optional notes about the transition.
    #[schemars(description = "Optional notes about the transition")]
    #[serde(default)]
    pub notes: Option<String>,

    /// Gate run IDs to link to this session.
    #[schemars(description = "Gate run IDs to append to this session's gate_runs list")]
    #[serde(default)]
    pub gate_run_ids: Vec<String>,
}

/// Parameters for the `session_list` tool.
#[derive(Deserialize, JsonSchema)]
pub struct SessionListParams {
    /// Filter by spec name.
    #[schemars(description = "Filter sessions by spec name (exact match)")]
    #[serde(default)]
    pub spec_name: Option<String>,

    /// Filter by session phase.
    #[schemars(
        description = "Filter by phase: 'created', 'agent_running', 'gate_evaluated', 'completed', or 'abandoned'"
    )]
    #[serde(default)]
    pub phase: Option<SessionPhase>,

    /// Maximum number of sessions to return (default: 20, max: 100).
    #[schemars(description = "Maximum sessions to return (default: 20, max: 100)")]
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Parameters for the `run_manifest` tool.
#[derive(Deserialize, JsonSchema)]
pub struct RunManifestParams {
    /// Path to the manifest TOML file.
    #[schemars(description = "Path to the run manifest TOML file (e.g. 'manifest.toml')")]
    pub manifest_path: String,

    /// Maximum seconds to wait for each agent subprocess (default: 600).
    #[schemars(
        description = "Maximum seconds per agent subprocess (default: 600). Applies to each session independently."
    )]
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Parameters for the `orchestrate_run` tool.
#[derive(Deserialize, JsonSchema)]
pub struct OrchestrateRunParams {
    /// Path to the manifest TOML file.
    #[schemars(
        description = "Path to the run manifest TOML file (e.g. 'manifest.toml'). \
        Must contain multi-session content (sessions with depends_on or more than one session)."
    )]
    pub manifest_path: String,

    /// Maximum seconds to wait for each agent subprocess (default: 600).
    #[schemars(
        description = "Maximum seconds per agent subprocess (default: 600). Applies to each session independently."
    )]
    #[serde(default)]
    pub timeout_secs: Option<u64>,

    /// Failure policy: "skip_dependents" (default) or "abort".
    #[schemars(description = "Failure policy for the orchestrated run. \
        'skip_dependents' (default): skip sessions that depend on failed ones, continue independent sessions. \
        'abort': stop dispatching new sessions on first failure.")]
    #[serde(default)]
    pub failure_policy: Option<String>,

    /// Merge strategy: "completion_time" (default) or "file_overlap".
    #[schemars(description = "Strategy for ordering session branches during merge. \
        'completion_time' (default): sort by completion timestamp. \
        'file_overlap': greedy pick sessions with least file overlap.")]
    #[serde(default)]
    pub merge_strategy: Option<String>,

    /// Conflict resolution mode: "auto" or "skip" (default).
    ///
    /// `auto`: use AI (Claude) to automatically resolve merge conflicts.
    /// `skip` (default): skip conflicting sessions without resolving.
    #[schemars(description = "Conflict resolution mode for the merge phase. \
        'auto': use AI (Claude) to automatically resolve merge conflicts — \
        requires the claude CLI to be available in PATH. \
        'skip' (default): skip conflicting sessions without resolving.")]
    #[serde(default)]
    pub conflict_resolution: Option<String>,
}

/// Parameters for the `orchestrate_status` tool.
#[derive(Deserialize, JsonSchema)]
pub struct OrchestrateStatusParams {
    /// Unique run ID returned by `orchestrate_run`.
    #[schemars(description = "Run ID from a previous orchestrate_run invocation (ULID string)")]
    pub run_id: String,
}

/// Response from the `orchestrate_run` tool.
#[derive(Serialize)]
struct OrchestrateRunResponse {
    /// Unique identifier for this orchestrated run.
    run_id: String,
    /// Total wall-clock duration in seconds.
    duration_secs: f64,
    /// Failure policy that was in effect.
    failure_policy: String,
    /// Per-session outcome summaries.
    sessions: Vec<OrchestrateSessionOutcome>,
    /// Aggregate counts.
    summary: OrchestrateRunSummary,
    /// Merge phase report (present if merge was attempted).
    #[serde(skip_serializing_if = "Option::is_none")]
    merge_report: Option<assay_types::MergeReport>,
}

/// Per-session outcome in an `orchestrate_run` response.
#[derive(Serialize)]
struct OrchestrateSessionOutcome {
    /// Session name.
    name: String,
    /// Outcome: "completed", "failed", "skipped".
    outcome: String,
    /// Error message (present for failed sessions).
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// Skip reason (present for skipped sessions).
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_reason: Option<String>,
}

/// Aggregate summary for an `orchestrate_run` response.
#[derive(Serialize)]
struct OrchestrateRunSummary {
    total: usize,
    completed: usize,
    failed: usize,
    skipped: usize,
}

/// Parameters for the `gate_evaluate` tool.
#[derive(Deserialize, JsonSchema)]
pub struct GateEvaluateParams {
    /// The spec whose agent criteria to evaluate.
    #[schemars(
        description = "Spec name to evaluate (filename without .toml extension, e.g. 'auth-flow'). \
        Evaluates all criteria using a headless Claude Code subprocess as an independent evaluator. \
        Replaces the gate_run/gate_report/gate_finalize flow for agent-evaluated criteria."
    )]
    pub name: String,

    /// Optional work session ID to auto-link results.
    #[schemars(
        description = "Work session ID (from session_create). When provided, the session's \
        worktree_path is used for diff computation, and the session is transitioned to gate_evaluated \
        with the gate run ID appended."
    )]
    #[serde(default)]
    pub session_id: Option<String>,

    /// Override evaluator timeout in seconds.
    #[schemars(
        description = "Evaluator subprocess timeout in seconds (overrides config, default: 120s). \
        This is separate from gate command timeouts — LLM inference has different latency characteristics."
    )]
    #[serde(default)]
    pub timeout: Option<u64>,

    /// Override evaluator model.
    #[schemars(
        description = "Model for the evaluator subprocess (overrides config, default: 'sonnet'). \
        Accepts model aliases like 'sonnet', 'opus' or full model names."
    )]
    #[serde(default)]
    pub model: Option<String>,
}

/// Parameters for the `milestone_list` tool.
#[derive(Deserialize, JsonSchema)]
pub struct MilestoneListParams {}

/// Parameters for the `milestone_get` tool.
#[derive(Deserialize, JsonSchema)]
pub struct MilestoneGetParams {
    #[schemars(description = "Milestone slug (filename without .toml, e.g. 'my-feature')")]
    pub slug: String,
}

/// Parameters for the `cycle_status` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CycleStatusParams {}

/// Parameters for the `cycle_advance` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CycleAdvanceParams {
    /// Optional milestone slug to advance. If omitted, targets the first in_progress milestone.
    #[serde(default)]
    pub milestone_slug: Option<String>,
}

/// Parameters for the `chunk_status` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChunkStatusParams {
    /// Slug of the chunk (spec) to report gate status for.
    pub chunk_slug: String,
}

// ── Response structs ─────────────────────────────────────────────────

/// A single entry in the `spec_list` response.
#[derive(Serialize)]
struct SpecListEntry {
    /// Spec name (filename stem without `.toml` extension).
    name: String,
    /// Human-readable description of the spec. Omitted from JSON when empty.
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    /// Number of criteria defined in the spec.
    criteria_count: usize,
    /// Spec format: `"legacy"` for flat `.toml` files, `"directory"` for directory-based specs.
    format: String,
    /// Whether a companion `spec.toml` (feature spec) exists. Omitted from JSON when `false`.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    has_feature_spec: bool,
}

/// Response envelope for `spec_list`, including scan errors.
#[derive(Serialize)]
struct SpecListResponse {
    /// Successfully parsed specs.
    specs: Vec<SpecListEntry>,
    /// Spec files that failed to parse. Omitted from JSON when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<SpecListError>,
}

/// A spec file that failed to parse during scanning.
#[derive(Serialize)]
struct SpecListError {
    /// Human-readable error description including the filename and parse error.
    message: String,
}

/// Aggregate gate run response returned by the `gate_run` tool.
#[derive(Serialize)]
struct GateRunResponse {
    /// Name of the spec that was evaluated.
    spec_name: String,
    /// Total number of criteria that passed.
    passed: usize,
    /// Total number of criteria that failed.
    failed: usize,
    /// Number of criteria skipped (descriptive-only, no command or path).
    skipped: usize,
    /// Number of required-enforcement criteria that passed.
    required_passed: usize,
    /// Number of required-enforcement criteria that failed.
    required_failed: usize,
    /// Number of advisory-enforcement criteria that passed.
    advisory_passed: usize,
    /// Number of advisory-enforcement criteria that failed.
    advisory_failed: usize,
    /// Whether the gate is blocked — `true` when any required criterion failed.
    blocked: bool,
    /// Total wall-clock duration for all evaluations in milliseconds.
    total_duration_ms: u64,
    /// Per-criterion results with status, enforcement, and optional evidence.
    criteria: Vec<CriterionSummary>,
    /// Session ID when agent criteria are present. Omitted for command-only specs.
    /// Use with `gate_report` and `gate_finalize` to complete agent evaluations.
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    /// Names of criteria pending agent evaluation. Omitted when no session is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_criteria: Option<Vec<String>>,
    /// Warnings about degraded operations (e.g., history save failure).
    /// Omitted from JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

/// Response from the `gate_report` tool confirming an evaluation was recorded.
#[derive(Serialize)]
struct GateReportResponse {
    /// Session ID the evaluation was submitted to.
    session_id: String,
    /// Name of the criterion that was evaluated.
    criterion_name: String,
    /// Whether the evaluation was accepted (always `true` on success).
    accepted: bool,
    /// Total number of evaluations recorded for this criterion (supports multiple evaluator roles).
    evaluations_count: usize,
    /// Names of criteria still pending agent evaluation in this session.
    pending_criteria: Vec<String>,
    /// Warnings about degraded operations (e.g., session state inconsistencies).
    /// Omitted from JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

/// Aggregate gate finalize response returned by the `gate_finalize` tool.
#[derive(Serialize)]
struct GateFinalizeResponse {
    /// Run ID of the finalized record.
    run_id: String,
    /// Name of the spec that was evaluated.
    spec_name: String,
    /// Total number of criteria that passed.
    passed: usize,
    /// Total number of criteria that failed.
    failed: usize,
    /// Number of criteria skipped.
    skipped: usize,
    /// Number of required-enforcement criteria that failed.
    required_failed: usize,
    /// Number of advisory-enforcement criteria that failed.
    advisory_failed: usize,
    /// Whether the gate is blocked — `true` when any required criterion failed.
    /// Consistent with `GateRunResponse` and `GateHistoryEntry`.
    blocked: bool,
    /// Whether the record was persisted to history.
    persisted: bool,
    /// Warnings about degraded operations (e.g., history save failure).
    /// Omitted from JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

/// Response for `gate_history` in list mode — returns recent run summaries.
#[derive(Serialize)]
struct GateHistoryListResponse {
    /// Spec name that was queried.
    spec_name: String,
    /// Total number of run files enumerated on disk for this spec (raw file count, before
    /// deserialization, outcome filtering, or limit are applied).
    total_runs: usize,
    /// Run summaries, most recent first.
    runs: Vec<GateHistoryEntry>,
    /// Warnings about degraded operations (e.g., unreadable history entries).
    /// Omitted from JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

/// A single run entry in the history list.
#[derive(Serialize)]
struct GateHistoryEntry {
    /// Unique run identifier.
    run_id: String,
    /// ISO 8601 timestamp of the run.
    timestamp: String,
    /// Number of criteria that passed.
    passed: usize,
    /// Number of criteria that failed.
    failed: usize,
    /// Number of criteria that were skipped.
    skipped: usize,
    /// Number of required criteria that passed.
    required_passed: usize,
    /// Number of required criteria that failed.
    required_failed: usize,
    /// Number of advisory criteria that passed.
    advisory_passed: usize,
    /// Number of advisory criteria that failed.
    advisory_failed: usize,
    /// Whether the gate was blocked (any required criterion failed).
    blocked: bool,
}

/// Response for the `chunk_status` tool — last gate run summary for a chunk.
#[derive(Debug, Serialize)]
struct ChunkStatusResponse {
    /// Slug of the queried chunk.
    chunk_slug: String,
    /// Whether any gate history exists for this chunk.
    has_history: bool,
    /// Run ID of the most recent gate run. `None` when `has_history` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_run_id: Option<String>,
    /// Number of criteria that passed in the latest run. `None` when `has_history` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    passed: Option<usize>,
    /// Number of criteria that failed in the latest run. `None` when `has_history` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    failed: Option<usize>,
    /// Number of required criteria that failed in the latest run. `None` when `has_history` is `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    required_failed: Option<usize>,
}

/// Per-criterion result within a gate run response.
#[derive(Serialize)]
struct CriterionSummary {
    /// Criterion name as defined in the spec.
    name: String,
    /// Evaluation status: `"passed"`, `"failed"`, or `"skipped"`.
    status: String,
    /// Enforcement level: `"required"` or `"advisory"`.
    enforcement: String,
    /// Criterion kind label: `"cmd"`, `"file"`, or `"agent"`. Omitted for skipped criteria.
    #[serde(skip_serializing_if = "Option::is_none")]
    kind_label: Option<String>,
    /// Process exit code. Present for command criteria, absent for file-exists and skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    /// Evaluation duration in milliseconds. Absent for skipped criteria.
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    /// First non-empty line of stderr for failed criteria. Absent for passed/skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    /// Full stdout output. Only present when `include_evidence=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<String>,
    /// Full stderr output. Only present when `include_evidence=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<String>,
    /// Whether stdout or stderr was truncated due to size limits.
    /// Absent for skipped criteria and when output was not truncated.
    #[serde(skip_serializing_if = "option_is_none_or_false")]
    truncated: Option<bool>,
    /// Original combined byte count before truncation.
    /// Absent when output was not truncated or criterion was skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    original_bytes: Option<u64>,
}

/// Returns `true` when the value is `None` or `Some(false)`.
fn option_is_none_or_false(v: &Option<bool>) -> bool {
    !matches!(v, Some(true))
}

/// Response from `session_create`.
#[derive(Serialize)]
struct SessionCreateResponse {
    /// The new session's unique ID (ULID).
    session_id: String,
    /// Spec name the session was created for.
    spec_name: String,
    /// Initial phase (always "created").
    phase: SessionPhase,
    /// ISO 8601 creation timestamp.
    created_at: chrono::DateTime<chrono::Utc>,
    /// Warnings about degraded operations.
    /// Future use: spec validation warnings, worktree path inaccessibility, etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

/// Response from `session_get`.
#[derive(Serialize)]
struct SessionGetResponse {
    /// Full session data.
    // CONSTRAINT: WorkSession must never add a `warnings` field — it would collide with the
    // response-level warnings below. If WorkSession needs warnings, wrap in a named `session` key instead.
    #[serde(flatten)]
    session: assay_types::work_session::WorkSession,
    /// Warnings about degraded operations.
    /// Future use: partial data, format migration issues, missing linked gate runs, etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

/// Response from `session_update`.
#[derive(Serialize)]
struct SessionUpdateResponse {
    /// Session ID that was updated.
    session_id: String,
    /// Previous phase.
    previous_phase: SessionPhase,
    /// New current phase.
    current_phase: SessionPhase,
    /// Number of gate run IDs now linked.
    gate_runs_count: usize,
    /// Warnings about degraded operations.
    /// Future use: gate run ID not found in history, duplicate link skipped, etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

/// A summary entry in the `session_list` response.
#[derive(Serialize)]
struct SessionListEntry {
    /// Session ID (ULID).
    id: String,
    /// Spec name.
    spec_name: String,
    /// Current phase.
    phase: SessionPhase,
    /// ISO 8601 creation timestamp.
    created_at: chrono::DateTime<chrono::Utc>,
    /// Number of gate runs linked.
    gate_runs_count: usize,
}

/// Response from `session_list`.
#[derive(Serialize)]
struct SessionListResponse {
    /// Total sessions on disk before filtering (includes all phases and specs).
    total_on_disk: usize,
    /// Matched sessions after filtering.
    sessions: Vec<SessionListEntry>,
    /// Warnings about degraded operations (e.g., unreadable session files).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

/// Per-criterion result within a gate_evaluate response.
#[derive(Serialize)]
struct EvaluateCriterionResult {
    /// Criterion name as defined in the spec.
    name: String,
    /// Evaluation outcome.
    outcome: assay_types::CriterionOutcome,
    /// Evaluator's reasoning for this judgment.
    reasoning: String,
    /// Concrete evidence observed. Omitted when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<String>,
    /// Enforcement level.
    enforcement: assay_types::Enforcement,
}

/// Per-session result in a `run_manifest` response.
#[derive(Serialize)]
struct RunManifestSessionResult {
    /// Spec name for this session.
    spec_name: String,
    /// Session ID assigned by the pipeline. Absent on early-stage failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    /// Outcome: "Success", "GateFailed", "MergeConflict", or "Error".
    outcome: String,
    /// Error details. Present only when outcome is "Error".
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RunManifestError>,
    /// Per-stage timing. Present on successful pipeline completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_timings: Option<Vec<RunManifestStageTiming>>,
}

/// Error detail for a failed pipeline session.
#[derive(Serialize)]
struct RunManifestError {
    /// Which pipeline stage failed.
    stage: String,
    /// Error description.
    message: String,
    /// Actionable recovery guidance.
    recovery: String,
    /// Elapsed seconds before failure.
    elapsed_secs: f64,
}

/// Timing for a single pipeline stage.
#[derive(Serialize)]
struct RunManifestStageTiming {
    /// Stage name.
    stage: String,
    /// Duration in seconds.
    duration_secs: f64,
}

/// Response from the `run_manifest` tool.
#[derive(Serialize)]
struct RunManifestResponse {
    /// Per-session results.
    sessions: Vec<RunManifestSessionResult>,
    /// Aggregate summary.
    summary: RunManifestSummary,
}

/// Aggregate summary for a `run_manifest` response.
#[derive(Serialize)]
struct RunManifestSummary {
    total: usize,
    succeeded: usize,
    gate_failed: usize,
    merge_conflict: usize,
    errored: usize,
}

/// Response from the `gate_evaluate` tool.
#[derive(Serialize)]
struct GateEvaluateResponse {
    /// Unique run ID for the persisted GateRunRecord.
    run_id: String,
    /// Name of the spec that was evaluated.
    spec_name: String,
    /// Aggregate summary: pass/fail counts and enforcement breakdown.
    summary: GateEvaluateSummary,
    /// Per-criterion results with outcome, reasoning, and evidence.
    results: Vec<EvaluateCriterionResult>,
    /// Whether the gate passed overall (from evaluator judgment).
    /// Note: this reflects the LLM's self-reported verdict. Check
    /// `summary.required_failed == 0` for the enforcement-derived result.
    overall_passed: bool,
    /// Model used for the evaluation.
    evaluator_model: String,
    /// Total handler wall-clock time in milliseconds (includes subprocess, git diff, and I/O).
    duration_ms: u64,
    /// Non-fatal warnings from evaluation, parse, history save, or session linking failures.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
    /// Linked session ID. Omitted when session_id was not provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    /// Truncation metadata for the diff. Present only when truncation occurred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    diff_truncation: Option<assay_types::DiffTruncation>,
}

/// Summary counts within a gate_evaluate response.
#[derive(Serialize)]
struct GateEvaluateSummary {
    /// Number of criteria that passed.
    passed: usize,
    /// Number of criteria that failed.
    failed: usize,
    /// Number of criteria that were skipped.
    skipped: usize,
    /// Number of required criteria that failed.
    required_failed: usize,
    /// Number of advisory criteria that failed.
    advisory_failed: usize,
    /// Whether the gate is blocked (any required criterion failed).
    blocked: bool,
}

/// Response from the `worktree_list` tool.
///
/// NOTE: `WorktreeInfo` fields partially duplicate `WorktreeStatus` fields (spec_slug, path, branch).
/// This is intentional — Info is the lightweight list entry, Status adds runtime state.
/// Unifying them would be a schema-breaking change deferred per D005.
#[derive(Serialize)]
struct WorktreeListResponse {
    /// The worktree entries found.
    entries: Vec<assay_types::WorktreeInfo>,
    /// Non-fatal warnings (e.g., prune failures). Omitted from JSON when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

// ── Constants ────────────────────────────────────────────────────────

/// Session timeout in seconds (30 minutes).
const SESSION_TIMEOUT_SECS: u64 = 1800;

/// Maximum byte size for captured git diff (32 KiB).
const DIFF_BUDGET_BYTES: usize = 32 * 1024;

/// Maximum number of timed-out session entries to retain (prevents unbounded growth).
const MAX_TIMED_OUT_ENTRIES: usize = 100;

// ── Timed-out session tracking ───────────────────────────────────────

/// Metadata about a session that was auto-finalized due to timeout.
/// Used to provide richer "not found" error messages.
#[derive(Debug, Clone)]
struct TimedOutInfo {
    spec_name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    timed_out_at: chrono::DateTime<chrono::Utc>,
    timeout_secs: u64,
}

// ── Server struct ────────────────────────────────────────────────────

/// MCP server exposing Assay spec, gate, context, worktree, and session operations as tools.
#[derive(Clone)]
pub struct AssayServer {
    tool_router: ToolRouter<Self>,
    sessions: Arc<Mutex<HashMap<String, GateEvalContext>>>,
    timed_out_sessions: Arc<Mutex<HashMap<String, TimedOutInfo>>>,
}

impl Default for AssayServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl AssayServer {
    /// Create a new server with the tool router initialized.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            timed_out_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// List all specs in the current Assay project.
    #[tool(
        description = "List all specs in the current Assay project. Returns {specs, errors?} where specs is an array of {name, description?, criteria_count, format, has_feature_spec?} objects and errors lists any spec files that failed to parse. Use this to discover available specs before calling spec_get or gate_run."
    )]
    pub async fn spec_list(&self) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };

        let specs_dir = cwd.join(".assay").join(&config.specs_dir);
        let scan_result = match assay_core::spec::scan(&specs_dir) {
            Ok(r) => r,
            Err(e) => return Ok(domain_error(&e)),
        };

        let specs: Vec<SpecListEntry> = scan_result
            .entries
            .iter()
            .map(|entry| match entry {
                SpecEntry::Legacy { slug, spec } => SpecListEntry {
                    name: slug.clone(),
                    description: spec.description.clone(),
                    criteria_count: spec.criteria.len(),
                    format: "legacy".to_string(),
                    has_feature_spec: false,
                },
                SpecEntry::Directory {
                    slug,
                    gates,
                    spec_path,
                } => SpecListEntry {
                    name: slug.clone(),
                    description: gates.description.clone(),
                    criteria_count: gates.criteria.len(),
                    format: "directory".to_string(),
                    has_feature_spec: spec_path.is_some(),
                },
            })
            .collect();

        let errors: Vec<SpecListError> = scan_result
            .errors
            .iter()
            .map(|e| SpecListError {
                message: e.to_string(),
            })
            .collect();

        let response = SpecListResponse { specs, errors };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get a spec by name.
    #[tool(
        description = "Get a spec by name. Returns the full spec definition as JSON. For legacy specs: {format, name, description, criteria}. For directory specs: {format, gates, feature_spec?}. Use spec_list first to find available spec names. Pass resolve=true to include effective timeout cascade (spec/config/default precedence) and working_dir validation."
    )]
    pub async fn spec_get(
        &self,
        params: Parameters<SpecGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };
        let entry = match load_spec_entry_mcp(&cwd, &config, &params.0.name) {
            Ok(e) => e,
            Err(err_result) => return Ok(err_result),
        };

        let resolved_block = if params.0.resolve {
            let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);
            let effective_timeout = config_timeout.unwrap_or(300);
            let working_dir = resolve_working_dir(&cwd, &config);
            Some(serde_json::json!({
                "timeout": {
                    "effective": effective_timeout,
                    "spec": serde_json::Value::Null,
                    "config": config_timeout,
                    "default": 300
                },
                "working_dir": {
                    "path": working_dir.to_string_lossy(),
                    "exists": working_dir.exists(),
                    "accessible": working_dir.is_dir()
                }
            }))
        } else {
            None
        };

        let json = match entry {
            SpecEntry::Legacy { spec, .. } => {
                let mut response = serde_json::json!({
                    "format": "legacy",
                    "name": spec.name,
                    "description": spec.description,
                    "criteria": spec.criteria,
                });
                if let (Some(resolved), Some(obj)) = (resolved_block, response.as_object_mut()) {
                    obj.insert("resolved".to_string(), resolved);
                }
                serde_json::to_string(&response)
            }
            SpecEntry::Directory {
                gates, spec_path, ..
            } => {
                let (feature_spec, feature_spec_error) = match spec_path {
                    Some(ref p) => match assay_core::spec::load_feature_spec(p) {
                        Ok(fs) => (Some(fs), None),
                        Err(e) => (None, Some(e.to_string())),
                    },
                    None => (None, None),
                };
                let mut response = serde_json::json!({
                    "format": "directory",
                    "gates": gates,
                    "feature_spec": feature_spec,
                });
                if let Some(err_msg) = feature_spec_error
                    && let Some(obj) = response.as_object_mut()
                {
                    obj.insert(
                        "feature_spec_error".to_string(),
                        serde_json::Value::String(err_msg),
                    );
                }
                if let (Some(resolved), Some(obj)) = (resolved_block, response.as_object_mut()) {
                    obj.insert("resolved".to_string(), resolved);
                }
                serde_json::to_string(&response)
            }
        }
        .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Statically validate a spec without running it.
    #[tool(
        description = "Statically validate a spec without running it. Returns a ValidationResult with \
            per-criterion diagnostics, severity levels (error/warning/info), and a summary. \
            Errors block validity (TOML parse errors, duplicate criterion names, missing required fields). \
            Warnings are advisory (missing AgentReport prompt, command not found on PATH). \
            When the spec being validated declares depends=[...], ALL specs are scanned for \
            dependency cycles involving this spec. Specs with no depends list skip cycle detection. \
            Use check_commands=true to verify command binaries exist on PATH."
    )]
    pub async fn spec_validate(
        &self,
        params: Parameters<SpecValidateParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };
        let specs_dir = cwd.join(".assay").join(&config.specs_dir);

        // Try to load the spec entry (handles TOML parse errors)
        let entry =
            match assay_core::spec::load_spec_entry_with_diagnostics(&params.0.name, &specs_dir) {
                Ok(entry) => entry,
                Err(assay_core::AssayError::SpecParse { message, .. })
                | Err(assay_core::AssayError::GatesSpecParse { message, .. })
                | Err(assay_core::AssayError::FeatureSpecParse { message, .. }) => {
                    // TOML parse error — return as ValidationResult with error diagnostic
                    let diagnostics = vec![assay_types::Diagnostic {
                        severity: assay_types::Severity::Error,
                        location: "toml".to_string(),
                        message,
                    }];
                    let summary = assay_types::DiagnosticSummary::from_diagnostics(&diagnostics);
                    let result = assay_types::ValidationResult {
                        spec: params.0.name.clone(),
                        valid: false,
                        diagnostics,
                        summary,
                    };
                    return Ok(CallToolResult::success(vec![Content::json(result)?]));
                }
                Err(assay_core::AssayError::SpecValidation { errors, .. })
                | Err(assay_core::AssayError::GatesSpecValidation { errors, .. })
                | Err(assay_core::AssayError::FeatureSpecValidation { errors, .. }) => {
                    let diagnostics =
                        assay_core::spec::validate::spec_errors_to_diagnostics(&errors);
                    let summary = assay_types::DiagnosticSummary::from_diagnostics(&diagnostics);
                    let result = assay_types::ValidationResult {
                        spec: params.0.name.clone(),
                        valid: false,
                        diagnostics,
                        summary,
                    };
                    return Ok(CallToolResult::success(vec![Content::json(result)?]));
                }
                Err(
                    ref e @ assay_core::AssayError::SpecNotFound { .. }
                    | ref e @ assay_core::AssayError::SpecNotFoundDiagnostic { .. },
                ) => {
                    let diagnostics = vec![assay_types::Diagnostic {
                        severity: assay_types::Severity::Error,
                        location: "name".to_string(),
                        message: e.to_string(),
                    }];
                    let summary = assay_types::DiagnosticSummary::from_diagnostics(&diagnostics);
                    let result = assay_types::ValidationResult {
                        spec: params.0.name.clone(),
                        valid: false,
                        diagnostics,
                        summary,
                    };
                    return Ok(CallToolResult::success(vec![Content::json(result)?]));
                }
                Err(other) => {
                    return Ok(domain_error(&other));
                }
            };

        // Spec loaded successfully — run validation with additional checks
        let result = assay_core::spec::validate::validate_spec_with_dependencies(
            &entry,
            params.0.check_commands,
            &specs_dir,
        );

        Ok(CallToolResult::success(vec![Content::json(result)?]))
    }

    /// Run quality gate checks for a spec.
    #[tool(
        description = "Run quality gate checks for a spec. Evaluates all executable criteria (shell commands, file checks) and returns pass/fail status per criterion with enforcement-level counts (required_passed, required_failed, advisory_passed, advisory_failed) and a blocked flag. Set timeout (seconds, default 300) to limit evaluation time. Set include_evidence=true for full stdout/stderr. If the spec contains agent-evaluated criteria (kind=AgentReport), a session_id and pending_criteria are returned — use gate_report for each, then gate_finalize to persist."
    )]
    pub async fn gate_run(
        &self,
        params: Parameters<GateRunParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };
        let entry = match load_spec_entry_mcp(&cwd, &config, &params.0.name) {
            Ok(e) => e,
            Err(err_result) => return Ok(err_result),
        };
        let include_evidence = params.0.include_evidence;
        if let Some(0) = params.0.timeout {
            return Ok(CallToolResult::error(vec![Content::text(
                "timeout must be greater than zero",
            )]));
        }
        let gate_timeout = Duration::from_secs(params.0.timeout.unwrap_or(300));

        let working_dir = resolve_working_dir(&cwd, &config);

        // Validate working directory exists before spawning evaluation.
        if !working_dir.is_dir() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "working directory does not exist or is not a directory: {}",
                working_dir.display()
            ))]));
        }

        let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);

        // Extract agent criteria info before moving entry into spawn_blocking.
        let agent_info = extract_agent_criteria_info(&entry);

        let eval_future = {
            let working_dir = working_dir.clone();
            tokio::task::spawn_blocking(move || match entry {
                SpecEntry::Legacy { spec, .. } => {
                    assay_core::gate::evaluate_all(&spec, &working_dir, None, config_timeout)
                }
                SpecEntry::Directory { gates, .. } => {
                    assay_core::gate::evaluate_all_gates(&gates, &working_dir, None, config_timeout)
                }
            })
        };

        let summary = match tokio::time::timeout(gate_timeout, eval_future).await {
            Ok(Ok(summary)) => summary,
            Ok(Err(e)) => {
                return Err(McpError::internal_error(
                    format!("gate evaluation panicked: {e}"),
                    None,
                ));
            }
            Err(_elapsed) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "gate evaluation timed out after {}s",
                    gate_timeout.as_secs()
                ))]));
            }
        };

        let mut response = format_gate_response(&summary, include_evidence);

        // If spec has agent criteria, create a session and attach to response.
        if let Some(info) = agent_info {
            // Capture git diff for the session (non-blocking on failure).
            let (diff, diff_truncated, diff_bytes_original) = {
                match std::process::Command::new("git")
                    .args(["diff", "HEAD"])
                    .current_dir(&working_dir)
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        let raw = String::from_utf8_lossy(&output.stdout);
                        assay_core::gate::truncate_diff(&raw, DIFF_BUDGET_BYTES)
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::warn!("git diff HEAD failed: {}", stderr.trim());
                        (None, false, None)
                    }
                    Err(e) => {
                        tracing::warn!("git diff HEAD command error: {e}");
                        (None, false, None)
                    }
                }
            };

            // Destructure summary to move fields without cloning Vec<CriterionResult>.
            let assay_types::GateRunSummary {
                spec_name, results, ..
            } = summary;
            let session = assay_core::gate::session::create_session(
                &spec_name,
                info.agent_criteria_names,
                info.spec_enforcement,
                results,
                diff,
                diff_truncated,
                diff_bytes_original,
            );

            let session_id = session.session_id.clone();
            let pending: Vec<String> = session.criteria_names.iter().cloned().collect();

            response.session_id = Some(session_id.clone());
            response.pending_criteria = Some(pending);

            // Store the session in memory and persist to disk (write-through).
            self.sessions
                .lock()
                .await
                .insert(session_id.clone(), session.clone());

            // Write-through: persist to disk so sessions survive restarts.
            if let Err(e) = assay_core::gate::session::save_context(&cwd.join(".assay"), &session) {
                tracing::warn!(
                    session_id = %session_id,
                    "gate_run: failed to persist session to disk: {e}"
                );
            }

            // Spawn timeout task.
            let sessions = Arc::clone(&self.sessions);
            let timed_out = Arc::clone(&self.timed_out_sessions);
            let assay_dir = cwd.join(".assay");
            let max_history = config.gates.as_ref().and_then(|g| g.max_history);
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(SESSION_TIMEOUT_SECS)).await;
                let session = {
                    let mut sessions = sessions.lock().await;
                    sessions.remove(&session_id)
                };
                if let Some(session) = session {
                    let timed_out_at = Utc::now();
                    tracing::warn!(
                        session_id = %session.session_id,
                        spec_name = %session.spec_name,
                        "session timed out after {}s, auto-finalizing",
                        SESSION_TIMEOUT_SECS
                    );

                    // Track the timed-out session for richer not-found messages.
                    {
                        let mut map = timed_out.lock().await;
                        if map.len() >= MAX_TIMED_OUT_ENTRIES {
                            // Evict the oldest entry by timed_out_at.
                            if let Some(oldest_key) = map
                                .iter()
                                .min_by_key(|(_, v)| v.timed_out_at)
                                .map(|(k, _)| k.clone())
                            {
                                map.remove(&oldest_key);
                            }
                        }
                        map.insert(
                            session.session_id.clone(),
                            TimedOutInfo {
                                spec_name: session.spec_name.clone(),
                                created_at: session.created_at,
                                timed_out_at,
                                timeout_secs: SESSION_TIMEOUT_SECS,
                            },
                        );
                    }

                    let record = assay_core::gate::session::finalize_as_timed_out(&session);
                    if let Err(e) = assay_core::history::save(&assay_dir, &record, max_history) {
                        tracing::error!(
                            session_id = %record.run_id,
                            "failed to save timed-out session: {e}"
                        );
                    } else {
                        tracing::info!(
                            session_id = %record.run_id,
                            spec_name = %record.summary.spec_name,
                            passed = record.summary.passed,
                            failed = record.summary.failed,
                            "timed-out session saved to history"
                        );
                    }
                }
            });
        } else {
            // Command-only spec (no agent criteria) — persist history immediately.
            // Extract spec_name for the tracing warn before moving summary into save_run.
            let spec_name_for_log = summary.spec_name.clone();
            let assay_dir = cwd.join(".assay");
            let max_history = config.gates.as_ref().and_then(|g| g.max_history);
            if let Err(e) = assay_core::history::save_run(
                &assay_dir,
                summary,
                Some(working_dir.to_string_lossy().to_string()),
                max_history,
            ) {
                let msg = format!("history save failed: {e}");
                tracing::warn!(
                    spec_name = %spec_name_for_log,
                    "{msg}"
                );
                response.warnings.push(msg);
            }
        }

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Evaluate all criteria via headless Claude Code subprocess.
    #[tool(
        description = "Evaluate all criteria for a spec using a headless Claude Code subprocess as an \
            independent evaluator. Returns per-criterion results with pass/fail/skip/warn outcome, \
            reasoning, and evidence. The evaluator subprocess runs with --tools \"\" and --max-turns 1 \
            (no tool use, single inference). Results are persisted as a GateRunRecord. \
            Replaces the gate_run/gate_report/gate_finalize flow for agent-evaluated criteria. \
            When session_id is provided, the session's worktree_path is used for diff computation \
            and the session is transitioned to gate_evaluated."
    )]
    pub async fn gate_evaluate(
        &self,
        params: Parameters<GateEvaluateParams>,
    ) -> Result<CallToolResult, McpError> {
        let start = std::time::Instant::now();
        let p = params.0;
        let mut warnings: Vec<String> = Vec::new();

        // ── Step 1: Load config and spec ──────────────────────────────
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };
        let entry = match load_spec_entry_mcp(&cwd, &config, &p.name) {
            Ok(e) => e,
            Err(err_result) => return Ok(err_result),
        };

        // Extract criteria, description, gate_section from the spec entry.
        let (spec_name, description, criteria, gate_section) = match &entry {
            SpecEntry::Legacy { spec, slug } => (
                slug.clone(),
                spec.description.clone(),
                spec.criteria.clone(),
                spec.gate.clone(),
            ),
            SpecEntry::Directory { slug, gates, .. } => (
                slug.clone(),
                gates.description.clone(),
                gates.criteria.clone(),
                gates.gate.clone(),
            ),
        };

        if criteria.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "spec has no criteria to evaluate",
            )]));
        }

        // ── Step 2: Resolve working directory ─────────────────────────
        let assay_dir = cwd.join(".assay");
        let (working_dir, linked_session_id) = if let Some(ref session_id) = p.session_id {
            let session = match {
                let assay_dir = assay_dir.clone();
                let session_id = session_id.clone();
                tokio::task::spawn_blocking(move || {
                    assay_core::work_session::load_session(&assay_dir, &session_id)
                })
            }
            .await
            .map_err(|e| McpError::internal_error(format!("task join failed: {e}"), None))?
            {
                Ok(s) => s,
                Err(e) => return Ok(domain_error(&e)),
            };
            (session.worktree_path.clone(), Some(session_id.clone()))
        } else {
            (resolve_working_dir(&cwd, &config), None)
        };

        if !working_dir.is_dir() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "working directory does not exist or is not a directory: {}",
                working_dir.display()
            ))]));
        }

        // ── Step 3: Resolve model and config (needed for context window lookup) ──
        let gates_config = config.gates.as_ref();
        let model = p.model.unwrap_or_else(|| {
            gates_config
                .map(|g| g.evaluator_model.clone())
                .unwrap_or_else(|| "sonnet".to_string())
        });

        // ── Step 4: Capture raw git diff ──────────────────────────────
        let raw_diff = {
            match std::process::Command::new("git")
                .args(["diff", "HEAD"])
                .current_dir(&working_dir)
                .output()
            {
                Ok(output) if output.status.success() => {
                    let raw = String::from_utf8_lossy(&output.stdout).into_owned();
                    if raw.is_empty() { None } else { Some(raw) }
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let msg = format!("git diff HEAD failed: {}", stderr.trim());
                    tracing::warn!("{msg}");
                    warnings.push(format!(
                        "{msg} — evaluator will assess without diff context"
                    ));
                    None
                }
                Err(e) => {
                    let msg = format!("git diff HEAD command error: {e}");
                    tracing::warn!("{msg}");
                    warnings.push(format!(
                        "{msg} — evaluator will assess without diff context"
                    ));
                    None
                }
            }
        };

        // ── Step 5: Build system prompt and schema ────────────────────
        let system_prompt = assay_core::evaluator::build_system_prompt();
        let schema_json = assay_core::evaluator::evaluator_schema_json();

        // Build criteria text for budget computation.
        // build_evaluator_prompt also builds criteria text internally — accept the
        // double computation (cheap for typical criterion counts, avoids refactoring
        // build_evaluator_prompt's API).
        let criteria_text: String = criteria
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let mut s = format!("### {}. {}\n{}", i + 1, c.name, c.description);
                if let Some(prompt) = &c.prompt {
                    s.push_str(&format!("\nEvaluation guidance: {prompt}"));
                }
                s
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        // ── Step 6: Token-budget the diff via budget_context ──────────
        let model_window = assay_core::context::context_window_for_model(Some(&model));

        let (diff, diff_truncation) = if let Some(ref raw) = raw_diff {
            // Count non-empty overhead inputs to determine the diff's expected
            // position in budget_context's canonical output order:
            // [system_prompt, spec_body, criteria_text, diff]
            let overhead_count = [&system_prompt, &description, &criteria_text]
                .iter()
                .filter(|s| !s.is_empty())
                .count();

            match assay_core::context::budget_context(
                &system_prompt,
                &description,
                &criteria_text,
                raw,
                model_window,
            ) {
                Ok(budgeted) => {
                    // Extract the diff by position: it occupies the slot after
                    // overhead items. If budget_context dropped it entirely,
                    // the vec will have fewer items than overhead_count + 1.
                    let truncated_diff = if budgeted.len() > overhead_count {
                        Some(budgeted[overhead_count].clone())
                    } else {
                        None // diff was fully dropped by the pipeline
                    };

                    let was_truncated = match &truncated_diff {
                        Some(d) => d.len() < raw.len(),
                        None => true, // diff was fully dropped
                    };

                    if was_truncated {
                        let raw_files = assay_core::gate::extract_diff_files(raw);
                        let kept_files = truncated_diff
                            .as_deref()
                            .map(assay_core::gate::extract_diff_files)
                            .unwrap_or_default();
                        let kept_set: std::collections::HashSet<&str> =
                            kept_files.iter().map(|s| s.as_str()).collect();
                        let omitted_files: Vec<String> = raw_files
                            .iter()
                            .filter(|f| !kept_set.contains(f.as_str()))
                            .cloned()
                            .collect();

                        let original_bytes = raw.len();
                        let truncated_bytes = truncated_diff.as_ref().map(|d| d.len()).unwrap_or(0);
                        let omitted_count = omitted_files.len();

                        if truncated_bytes == 0 {
                            warnings.push(format!(
                                "Diff entirely omitted ({original_bytes} bytes, \
                                 {omitted_count} files) — token budget exceeded; \
                                 evaluator will assess without diff context"
                            ));
                        } else {
                            warnings.push(format!(
                                "Diff truncated from {original_bytes} to \
                                 {truncated_bytes} bytes ({} files kept, \
                                 {omitted_count} files omitted) to fit token budget",
                                kept_files.len()
                            ));
                        }

                        debug_assert!(
                            truncated_bytes <= original_bytes,
                            "truncated_bytes ({truncated_bytes}) > original_bytes ({original_bytes})"
                        );

                        let meta = assay_types::DiffTruncation {
                            original_bytes: original_bytes as u64,
                            truncated_bytes: truncated_bytes as u64,
                            included_files: kept_files,
                            omitted_files,
                        };
                        (truncated_diff, Some(meta))
                    } else {
                        (truncated_diff.or_else(|| Some(raw.clone())), None)
                    }
                }
                Err(e) => {
                    tracing::warn!("budget_context failed: {e}");
                    // Fall back to byte-budget truncation as a safety net so the
                    // evaluator doesn't receive an unbounded diff.
                    let (truncated, _was_truncated, _original_bytes) =
                        assay_core::gate::truncate_diff(raw, DIFF_BUDGET_BYTES);
                    warnings.push(format!(
                        "Diff budget computation failed: {e} — \
                         diff byte-truncated to {DIFF_BUDGET_BYTES} as fallback"
                    ));
                    (truncated, None)
                }
            }
        } else {
            (None, None)
        };

        // ── Step 7: Build evaluator prompt ────────────────────────────
        // Criterion prompts are inlined by build_evaluator_prompt as
        // "Evaluation guidance" under each criterion — no separate
        // agent_prompt needed to avoid duplication.
        let prompt = assay_core::evaluator::build_evaluator_prompt(
            &spec_name,
            &description,
            &criteria,
            diff.as_deref(),
            None,
        );

        // ── Step 8: Construct EvaluatorConfig ─────────────────────────
        if let Some(0) = p.timeout {
            return Ok(CallToolResult::error(vec![Content::text(
                "timeout must be greater than zero",
            )]));
        }
        let timeout = Duration::from_secs(
            p.timeout
                .unwrap_or_else(|| gates_config.map(|g| g.evaluator_timeout).unwrap_or(120)),
        );
        let retries = gates_config.map(|g| g.evaluator_retries).unwrap_or(1);

        let eval_config = assay_core::evaluator::EvaluatorConfig {
            model: model.clone(),
            timeout,
            retries,
        };

        // ── Step 9: Spawn evaluator subprocess ────────────────────────
        let evaluator_result = match assay_core::evaluator::run_evaluator(
            &prompt,
            &system_prompt,
            schema_json,
            &eval_config,
            &working_dir,
        )
        .await
        {
            Ok(r) => r,
            Err(assay_core::error::EvaluatorError::NotInstalled) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Claude Code CLI (`claude`) not found in PATH. \
                     Install from https://claude.ai/code to use gate_evaluate.",
                )]));
            }
            Err(assay_core::error::EvaluatorError::Timeout { timeout_secs }) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Evaluator timed out after {timeout_secs}s. \
                     Increase timeout or reduce criteria count."
                ))]));
            }
            Err(assay_core::error::EvaluatorError::Crash { exit_code, stderr }) => {
                let code_str = exit_code
                    .map(|c| format!(" (exit code: {c})"))
                    .unwrap_or_default();
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Evaluator subprocess crashed{code_str}: {}",
                    truncate_to_char_boundary(&stderr, 500)
                ))]));
            }
            Err(assay_core::error::EvaluatorError::ParseError { raw_output, error }) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Evaluator output parse error: {error}. Raw output: {}",
                    truncate_to_char_boundary(&raw_output, 500)
                ))]));
            }
            Err(assay_core::error::EvaluatorError::NoStructuredOutput { raw_output }) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Evaluator output missing structured_output. Raw output: {}",
                    truncate_to_char_boundary(&raw_output, 500)
                ))]));
            }
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Evaluator failed: {e}"
                ))]));
            }
        };

        // Merge evaluator parse warnings.
        warnings.extend(evaluator_result.warnings);

        // ── Step 10: Map to GateRunRecord ─────────────────────────────
        let enforcement_map: HashMap<String, assay_types::Enforcement> = criteria
            .iter()
            .map(|c| {
                let resolved =
                    assay_core::gate::resolve_enforcement(c.enforcement, gate_section.as_ref());
                (c.name.clone(), resolved)
            })
            .collect();

        let (mut record, map_warnings) = assay_core::evaluator::map_evaluator_output(
            &spec_name,
            &evaluator_result.output,
            &enforcement_map,
            start.elapsed(),
        );
        warnings.extend(map_warnings);

        let run_id = record.run_id.clone();
        let overall_passed = evaluator_result.output.summary.passed;
        let duration_ms = record.summary.total_duration_ms;

        // Build response results from the evaluator output.
        let results: Vec<EvaluateCriterionResult> = evaluator_result
            .output
            .criteria
            .iter()
            .map(|cr| {
                let enforcement = enforcement_map
                    .get(&cr.name)
                    .copied()
                    .unwrap_or(assay_types::Enforcement::Required);
                EvaluateCriterionResult {
                    name: cr.name.clone(),
                    outcome: cr.outcome,
                    reasoning: cr.reasoning.clone(),
                    evidence: cr.evidence.clone(),
                    enforcement,
                }
            })
            .collect();

        // ── Step 11: Persist via history::save ────────────────────────
        // Attach truncation metadata before cloning for save so the
        // persisted record includes it. The response takes it from the
        // original `diff_truncation` binding (no extra clone needed).
        record.diff_truncation = diff_truncation.clone();

        let max_history = gates_config.and_then(|g| g.max_history);
        match {
            let assay_dir = assay_dir.clone();
            let record = record.clone();
            tokio::task::spawn_blocking(move || {
                assay_core::history::save(&assay_dir, &record, max_history)
            })
        }
        .await
        .map_err(|e| McpError::internal_error(format!("task join failed: {e}"), None))?
        {
            Ok(_) => {}
            Err(e) => {
                let msg = format!("history save failed: {e}");
                tracing::warn!(run_id = %run_id, "{msg}");
                warnings.push(msg);
            }
        }

        // ── Step 12: Session auto-linking ─────────────────────────────
        if let Some(ref session_id) = linked_session_id {
            let notes = format!(
                "gate_evaluate: {}",
                if overall_passed { "passed" } else { "failed" }
            );
            match {
                let assay_dir = assay_dir.clone();
                let session_id = session_id.clone();
                let run_id = run_id.clone();
                tokio::task::spawn_blocking(move || {
                    assay_core::work_session::record_gate_result(
                        &assay_dir,
                        &session_id,
                        &run_id,
                        "gate_evaluate",
                        Some(&notes),
                    )
                })
            }
            .await
            .map_err(|e| McpError::internal_error(format!("task join failed: {e}"), None))?
            {
                Ok(_) => {}
                Err(e) => {
                    let msg = format!("session auto-link failed: {e}");
                    tracing::warn!(session_id = %session_id, "{msg}");
                    warnings.push(msg);
                }
            }
        }

        // ── Build response ────────────────────────────────────────────
        let response = GateEvaluateResponse {
            run_id,
            spec_name,
            summary: GateEvaluateSummary {
                passed: record.summary.passed,
                failed: record.summary.failed,
                skipped: record.summary.skipped,
                required_failed: record.summary.enforcement.required_failed,
                advisory_failed: record.summary.enforcement.advisory_failed,
                blocked: record.summary.enforcement.required_failed > 0,
            },
            results,
            overall_passed,
            evaluator_model: model,
            duration_ms,
            warnings,
            session_id: linked_session_id,
            diff_truncation,
        };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Submit an agent evaluation for a single criterion in an active gate session.
    #[tool(
        description = "Submit an agent evaluation for a single criterion in an active gate session. Call gate_run first to start a session, then gate_report for each agent-evaluated criterion, then gate_finalize to persist results."
    )]
    pub async fn gate_report(
        &self,
        params: Parameters<GateReportParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let mut sessions = self.sessions.lock().await;
        let Some(session) = sessions.get_mut(&p.session_id) else {
            // Drop the sessions lock before awaiting the helper (it acquires timed_out lock).
            drop(sessions);
            return Ok(self.session_not_found_error(&p.session_id).await);
        };

        let eval = AgentEvaluation {
            passed: p.passed,
            evidence: p.evidence,
            reasoning: p.reasoning,
            confidence: p.confidence,
            evaluator_role: p.evaluator_role,
            timestamp: Utc::now(),
        };

        if let Err(e) =
            assay_core::gate::session::report_evaluation(session, &p.criterion_name, eval)
        {
            return Ok(domain_error(&e));
        }

        let pending: Vec<String> = session
            .criteria_names
            .iter()
            .filter(|name| !session.agent_evaluations.contains_key(*name))
            .cloned()
            .collect();
        let evaluations_count = session
            .agent_evaluations
            .get(&p.criterion_name)
            .map_or(0, |v| v.len());

        // Write-through: persist updated session to disk.
        let mut warnings = Vec::new();
        {
            let cwd = resolve_cwd()?;
            if let Err(e) = assay_core::gate::session::save_context(&cwd.join(".assay"), session) {
                let msg = format!("gate_report: failed to persist session to disk: {e}");
                tracing::warn!(session_id = %p.session_id, "{msg}");
                warnings.push(msg);
            }
        }

        let response = GateReportResponse {
            session_id: p.session_id,
            criterion_name: p.criterion_name,
            accepted: true,
            evaluations_count,
            pending_criteria: pending,
            warnings,
        };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Finalize an active gate session, persisting all accumulated evaluations.
    #[tool(
        description = "Finalize an active gate session, persisting all accumulated evaluations as a GateRunRecord. Missing required criteria cause the gate to fail. Call this after submitting all gate_report evaluations."
    )]
    pub async fn gate_finalize(
        &self,
        params: Parameters<GateFinalizeParams>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = params.0.session_id;

        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(e) => return Ok(e),
        };
        let assay_dir = cwd.join(".assay");

        // Try in-memory first, then fall back to disk load.
        let session = {
            let mut sessions = self.sessions.lock().await;
            sessions.remove(&session_id)
        };
        let session = match session {
            Some(s) => s,
            None => {
                // Fallback: try loading from persisted disk state.
                match assay_core::gate::session::load_context(&assay_dir, &session_id) {
                    Ok(s) => {
                        tracing::info!(
                            session_id = %session_id,
                            "gate_finalize: recovered session from disk (not in memory)"
                        );
                        s
                    }
                    Err(assay_core::AssayError::GateEvalContextNotFound { .. }) => {
                        return Ok(self.session_not_found_error(&session_id).await);
                    }
                    Err(e) => {
                        tracing::warn!(
                            session_id = %session_id,
                            "gate_finalize: disk load failed: {e}"
                        );
                        return Ok(self.session_not_found_error(&session_id).await);
                    }
                }
            }
        };

        let working_dir = resolve_working_dir(&cwd, &config);
        let max_history = config.gates.as_ref().and_then(|g| g.max_history);

        let record = assay_core::gate::session::build_finalized_record(
            &session,
            Some(&working_dir.to_string_lossy()),
        );

        let mut warnings = Vec::new();
        let persisted = if let Err(e) = assay_core::history::save(&assay_dir, &record, max_history)
        {
            let msg = format!("history save failed: {e}");
            tracing::warn!(session_id = %record.run_id, "{msg}");
            warnings.push(msg);
            false
        } else {
            true
        };

        // Clean up on-disk session file (best-effort).
        let disk_path = assay_dir
            .join("gate_sessions")
            .join(format!("{}.json", session_id));
        if disk_path.exists()
            && let Err(e) = std::fs::remove_file(&disk_path)
        {
            tracing::warn!(
                session_id = %session_id,
                path = %disk_path.display(),
                "gate_finalize: failed to clean up on-disk session: {e}"
            );
        }

        let required_failed = record.summary.enforcement.required_failed;
        let response = GateFinalizeResponse {
            run_id: record.run_id,
            spec_name: record.summary.spec_name,
            passed: record.summary.passed,
            failed: record.summary.failed,
            skipped: record.summary.skipped,
            required_failed,
            advisory_failed: record.summary.enforcement.advisory_failed,
            blocked: required_failed > 0,
            persisted,
            warnings,
        };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Query gate run history for a spec.
    #[tool(
        description = "Query gate run history for a spec. Without run_id, returns a list of recent runs with summary counts (filterable by outcome: passed/failed/any). With run_id, returns the full gate run record including all criterion results. Use this to check past gate outcomes and track quality trends."
    )]
    pub async fn gate_history(
        &self,
        params: Parameters<GateHistoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let assay_dir = cwd.join(".assay");

        // Detail mode: return full record for a specific run.
        if let Some(ref run_id) = params.0.run_id {
            let record = match assay_core::history::load(&assay_dir, &params.0.name, run_id) {
                Ok(r) => r,
                Err(e) => return Ok(domain_error(&e)),
            };
            let json = serde_json::to_string(&record).map_err(|e| {
                McpError::internal_error(format!("serialization failed: {e}"), None)
            })?;
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        // List mode: return recent run summaries.
        let all_ids = match assay_core::history::list(&assay_dir, &params.0.name) {
            Ok(ids) => ids,
            Err(e) => return Ok(domain_error(&e)),
        };

        let total_runs = all_ids.len();
        let limit = params.0.limit.unwrap_or(10).min(50);
        let outcome_filter = params.0.outcome.as_deref().unwrap_or("any");

        // Validate outcome filter value.
        if !matches!(outcome_filter, "passed" | "failed" | "any") {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "unrecognized outcome filter '{}': valid values are \"passed\", \"failed\", \"any\"",
                outcome_filter
            ))]));
        }

        // Iterate newest-first, loading and filtering by outcome, collecting up to `limit` matches.
        let mut runs = Vec::with_capacity(limit);
        let mut warnings = Vec::new();
        for id in all_ids.iter().rev() {
            if runs.len() >= limit {
                break;
            }
            match assay_core::history::load(&assay_dir, &params.0.name, id) {
                Ok(record) => {
                    let is_failed = record.summary.enforcement.required_failed > 0;
                    let matches = match outcome_filter {
                        "passed" => !is_failed,
                        "failed" => is_failed,
                        _ => true, // "any" (validated above)
                    };
                    if matches {
                        runs.push(GateHistoryEntry {
                            run_id: record.run_id,
                            timestamp: record.timestamp.to_rfc3339(),
                            passed: record.summary.passed,
                            failed: record.summary.failed,
                            skipped: record.summary.skipped,
                            required_passed: record.summary.enforcement.required_passed,
                            required_failed: record.summary.enforcement.required_failed,
                            advisory_passed: record.summary.enforcement.advisory_passed,
                            advisory_failed: record.summary.enforcement.advisory_failed,
                            blocked: is_failed,
                        });
                    }
                }
                Err(e) => {
                    let msg = format!("skipping unreadable history entry '{}': {e}", id);
                    tracing::warn!(run_id = %id, "{msg}");
                    warnings.push(msg);
                }
            }
        }

        let response = GateHistoryListResponse {
            spec_name: params.0.name,
            total_runs,
            runs,
            warnings,
        };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Diagnose token usage and bloat in a Claude Code session.
    #[tool(
        description = "Diagnose token usage, bloat breakdown, and context utilization for a Claude Code session. \
            Returns a full DiagnosticsReport with entry counts, usage data, bloat categories, and utilization percentage. \
            Omit session_id to diagnose the most recent session for this project."
    )]
    pub async fn context_diagnose(
        &self,
        params: Parameters<ContextDiagnoseParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let session_id = params.0.session_id;

        let report = tokio::task::spawn_blocking(move || {
            let session_dir = assay_core::context::find_session_dir(&cwd)?;
            let session_path =
                assay_core::context::resolve_session(&session_dir, session_id.as_deref())?;
            let file_session_id = session_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            assay_core::context::diagnose(&session_path, &file_session_id)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("task join failed: {e}"), None))?;

        match report {
            Ok(report) => {
                let json = serde_json::to_string(&report).map_err(|e| {
                    McpError::internal_error(format!("serialization failed: {e}"), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(domain_error(&e)),
        }
    }

    /// Estimate current token usage and context window health.
    #[tool(
        description = "Estimate current token usage and context window health for a Claude Code session. \
            Returns context tokens, output tokens, utilization percentage, and a health indicator \
            (healthy/warning/critical). When 5+ assistant turns exist, includes growth_rate with \
            avg_tokens_per_turn, estimated_turns_remaining, and turn_count. \
            Omit session_id to estimate the most recent session for this project."
    )]
    pub async fn estimate_tokens(
        &self,
        params: Parameters<EstimateTokensParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let session_id = params.0.session_id;

        let estimate = tokio::task::spawn_blocking(move || {
            let session_dir = assay_core::context::find_session_dir(&cwd)?;
            let session_path =
                assay_core::context::resolve_session(&session_dir, session_id.as_deref())?;
            let file_session_id = session_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            assay_core::context::estimate_tokens(&session_path, &file_session_id)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("task join failed: {e}"), None))?;

        match estimate {
            Ok(estimate) => {
                let json = serde_json::to_string(&estimate).map_err(|e| {
                    McpError::internal_error(format!("serialization failed: {e}"), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(domain_error(&e)),
        }
    }

    /// Create an isolated git worktree for a spec.
    #[tool(
        description = "Create an isolated git worktree for a spec. Returns WorktreeInfo with spec_slug, path, branch, and base_branch. Validates that the spec exists before creating the worktree."
    )]
    pub async fn worktree_create(
        &self,
        params: Parameters<WorktreeCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };

        let worktree_dir = assay_core::worktree::resolve_worktree_dir(
            params.0.worktree_dir.as_deref(),
            &config,
            &cwd,
        );
        let specs_dir = cwd.join(".assay").join(&config.specs_dir);

        let info = match assay_core::worktree::create(
            &cwd,
            &params.0.name,
            params.0.base.as_deref(),
            &worktree_dir,
            &specs_dir,
            None, // session linkage from MCP is future work
        ) {
            Ok(info) => info,
            Err(e) => return Ok(domain_error(&e)),
        };

        let json = serde_json::to_string(&info)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// List all active assay worktrees.
    #[tool(
        description = "List all active assay-managed worktrees. Returns an array of WorktreeInfo objects with spec_slug, path, and branch for each worktree."
    )]
    pub async fn worktree_list(
        &self,
        #[allow(unused_variables)] params: Parameters<WorktreeListParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        // Validate project context — ensures we are inside an Assay project before
        // listing worktrees, consistent with other worktree tools.
        if let Err(err_result) = load_config(&cwd) {
            return Ok(err_result);
        }

        let result = match assay_core::worktree::list(&cwd) {
            Ok(r) => r,
            Err(e) => return Ok(domain_error(&e)),
        };

        let response = WorktreeListResponse {
            entries: result.entries,
            warnings: result.warnings,
        };
        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Check worktree status (branch, dirty state, ahead/behind relative to base branch).
    #[tool(
        description = "Check worktree status including branch, HEAD commit, dirty state, and ahead/behind counts relative to the base branch. Set fetch=true to update remote refs first. Returns a WorktreeStatus object with optional warnings."
    )]
    pub async fn worktree_status(
        &self,
        params: Parameters<WorktreeStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };

        let worktree_dir = assay_core::worktree::resolve_worktree_dir(
            params.0.worktree_dir.as_deref(),
            &config,
            &cwd,
        );
        let worktree_path = worktree_dir.join(&params.0.name);

        // Optionally fetch the base branch from the remote before computing status
        if params.0.fetch.unwrap_or(false)
            && let Some(metadata) = assay_core::worktree::read_metadata(&worktree_path)
        {
            // Best-effort fetch — ignore failures (e.g., offline, no remote)
            let _ = std::process::Command::new("git")
                .args(["fetch", "origin", &metadata.base_branch])
                .current_dir(&worktree_path)
                .output();
        }

        let status = match assay_core::worktree::status(&worktree_path, &params.0.name) {
            Ok(s) => s,
            Err(e) => return Ok(domain_error(&e)),
        };

        let json = serde_json::to_string(&status)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // TODO(M002): worktree_cleanup_all tool — deferred per D005 (MCP additive-only)

    /// Remove a worktree and its associated branch.
    #[tool(
        description = "Remove a worktree and its associated branch. Defaults to force=true since MCP tools are non-interactive. Returns a confirmation object with the removed spec name."
    )]
    pub async fn worktree_cleanup(
        &self,
        params: Parameters<WorktreeCleanupParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };

        let worktree_dir = assay_core::worktree::resolve_worktree_dir(
            params.0.worktree_dir.as_deref(),
            &config,
            &cwd,
        );
        let worktree_path = worktree_dir.join(&params.0.name);

        // Default force=true for MCP (non-interactive agent context)
        let force = params.0.force.unwrap_or(true);

        if let Err(e) = assay_core::worktree::cleanup(&cwd, &worktree_path, &params.0.name, force) {
            return Ok(domain_error(&e));
        }

        let response = serde_json::json!({"removed": params.0.name});
        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ── Merge tools ─────────────────────────────────────────────────

    /// Check for merge conflicts between two refs using git merge-tree (read-only, zero side effects).
    #[tool(
        description = "Check for merge conflicts between two git refs. Uses `git merge-tree --write-tree` — read-only with zero side effects (no index mutation, no working tree changes). Returns MergeCheck with clean/conflicts status, file changes, ahead/behind counts, and fast-forward detection. Does not require an Assay project — works in any git repository."
    )]
    pub async fn merge_check(
        &self,
        params: Parameters<MergeCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;

        let result = match assay_core::merge::merge_check(
            &cwd,
            &params.0.base,
            &params.0.head,
            params.0.max_conflicts,
        ) {
            Ok(check) => check,
            Err(e) => return Ok(domain_error(&e)),
        };

        let json = serde_json::to_string(&result)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ── Session tools ────────────────────────────────────────────────

    /// Create a new work session for a spec.
    #[tool(
        description = "Create a new work session for a spec. Returns the session ID and initial state. \
            The session starts in 'created' phase. Use session_update to advance through phases \
            (agent_running → gate_evaluated → completed) or abandon. Validates that the spec exists."
    )]
    pub async fn session_create(
        &self,
        params: Parameters<SessionCreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };

        // Validate spec exists (consistent with gate_run)
        if let Err(err_result) = load_spec_entry_mcp(&cwd, &config, &params.0.spec_name) {
            return Ok(err_result);
        }

        let assay_dir = cwd.join(".assay");
        let session = assay_core::work_session::create_work_session(
            &params.0.spec_name,
            params.0.worktree_path,
            &params.0.agent_command,
            params.0.agent_model.as_deref(),
        );

        if let Err(e) = assay_core::work_session::save_session(&assay_dir, &session) {
            return Ok(domain_error(&e));
        }

        let response = SessionCreateResponse {
            session_id: session.id,
            spec_name: session.spec_name,
            phase: session.phase,
            created_at: session.created_at,
            warnings: Vec::new(),
        };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get a work session by ID.
    #[tool(
        description = "Get a work session by ID. Returns the full session including phase, transitions, \
            agent info, and linked gate runs. Use session_list to find session IDs."
    )]
    pub async fn session_get(
        &self,
        params: Parameters<SessionGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let assay_dir = cwd.join(".assay");

        let session = match assay_core::work_session::load_session(&assay_dir, &params.0.session_id)
        {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("{e}. Use session_list to find valid session IDs.");
                return Ok(CallToolResult::error(vec![Content::text(msg)]));
            }
        };

        let response = SessionGetResponse {
            session,
            warnings: Vec::new(),
        };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Update a work session: transition phase and/or link gate run IDs.
    #[tool(
        description = "Update a work session: transition phase and/or link gate run IDs. \
            Valid transitions: created→agent_running, agent_running→gate_evaluated, \
            gate_evaluated→completed, any non-terminal→abandoned. \
            Terminal phases (completed, abandoned) cannot be transitioned out of. \
            Invalid transitions are rejected with a clear error message."
    )]
    pub async fn session_update(
        &self,
        params: Parameters<SessionUpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let assay_dir = cwd.join(".assay");

        let gate_run_ids = params.0.gate_run_ids.clone();

        // Capture the phase before mutation so we can include it in the response.
        let previous_phase =
            match assay_core::work_session::load_session(&assay_dir, &params.0.session_id) {
                Ok(s) => s.phase,
                Err(e) => {
                    let msg =
                        if matches!(e, assay_core::error::AssayError::WorkSessionNotFound { .. }) {
                            format!("{e}. Use session_list to find valid session IDs.")
                        } else {
                            format!("{e}")
                        };
                    return Ok(CallToolResult::error(vec![Content::text(msg)]));
                }
            };

        let session = match assay_core::work_session::with_session(
            &assay_dir,
            &params.0.session_id,
            |session| {
                assay_core::work_session::transition_session(
                    session,
                    params.0.phase,
                    &params.0.trigger,
                    params.0.notes.as_deref(),
                )?;
                for id in &gate_run_ids {
                    if !session.gate_runs.contains(id) {
                        session.gate_runs.push(id.clone());
                    }
                }
                Ok(())
            },
        ) {
            Ok(s) => s,
            Err(e) => {
                let msg = if matches!(e, assay_core::error::AssayError::WorkSessionNotFound { .. })
                {
                    format!("{e}. Use session_list to find valid session IDs.")
                } else {
                    format!("{e}")
                };
                return Ok(CallToolResult::error(vec![Content::text(msg)]));
            }
        };

        let response = SessionUpdateResponse {
            session_id: session.id,
            previous_phase,
            current_phase: session.phase,
            gate_runs_count: session.gate_runs.len(),
            warnings: Vec::new(),
        };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// List work sessions with optional filters.
    #[tool(
        description = "List work sessions with optional filters. Returns summary entries \
            (id, spec_name, phase, created_at, gate_runs_count). \
            Filter by spec_name (exact match) and/or status (phase). \
            Results are ordered chronologically (oldest first, ULID order)."
    )]
    pub async fn session_list(
        &self,
        params: Parameters<SessionListParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let assay_dir = cwd.join(".assay");
        let mut warnings = Vec::new();

        let ids = match assay_core::work_session::list_sessions(&assay_dir) {
            Ok(ids) => ids,
            Err(e) => return Ok(domain_error(&e)),
        };

        let total = ids.len();
        let limit = params.0.limit.unwrap_or(20).clamp(1, 100);

        let mut sessions = Vec::new();
        for id in &ids {
            let session = match assay_core::work_session::load_session(&assay_dir, id) {
                Ok(s) => s,
                Err(e) => {
                    warnings.push(format!("skipping session {id}: {e}"));
                    continue;
                }
            };

            // Apply filters
            if let Some(ref spec_filter) = params.0.spec_name
                && session.spec_name != *spec_filter
            {
                continue;
            }
            if let Some(phase_filter) = params.0.phase
                && session.phase != phase_filter
            {
                continue;
            }

            sessions.push(SessionListEntry {
                id: session.id,
                spec_name: session.spec_name,
                phase: session.phase,
                created_at: session.created_at,
                gate_runs_count: session.gate_runs.len(),
            });

            if sessions.len() >= limit {
                break;
            }
        }

        let response = SessionListResponse {
            total_on_disk: total,
            sessions,
            warnings,
        };

        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Run a manifest through the end-to-end pipeline.
    #[tool(
        description = "Run a manifest through the end-to-end pipeline: spec load → worktree create → harness config → agent launch → gate evaluate → merge check. \
            Returns per-session results with outcomes, stage timings, and structured error details. \
            Each session runs sequentially. One session's failure does not block subsequent sessions. \
            The sync pipeline is wrapped in spawn_blocking per D007."
    )]
    pub async fn run_manifest(
        &self,
        params: Parameters<RunManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };

        let manifest_path = PathBuf::from(&params.0.manifest_path);
        let manifest_path = if manifest_path.is_absolute() {
            manifest_path
        } else {
            cwd.join(&manifest_path)
        };

        let specs_dir = cwd.join(".assay").join(&config.specs_dir);
        let assay_dir = cwd.join(".assay");
        let worktree_base = assay_core::worktree::resolve_worktree_dir(None, &config, &cwd);
        let timeout_secs = params
            .0
            .timeout_secs
            .unwrap_or(assay_core::pipeline::PipelineConfig::DEFAULT_TIMEOUT_SECS);

        // Load manifest (sync, cheap)
        let manifest = match assay_core::manifest::load(&manifest_path) {
            Ok(m) => m,
            Err(e) => return Ok(domain_error(&e)),
        };

        let session_specs: Vec<String> = manifest.sessions.iter().map(|s| s.spec.clone()).collect();

        let pipeline_config = assay_core::pipeline::PipelineConfig {
            project_root: cwd.clone(),
            assay_dir,
            specs_dir,
            worktree_base,
            timeout_secs,
            base_branch: None,
        };

        // Wrap the sync pipeline in spawn_blocking (D007).
        let results = tokio::task::spawn_blocking(move || {
            let harness_writer: Box<assay_core::pipeline::HarnessWriter> = Box::new(
                |profile: &assay_types::HarnessProfile, worktree_path: &std::path::Path| {
                    let claude_config = assay_harness::claude::generate_config(profile);
                    assay_harness::claude::write_config(&claude_config, worktree_path)
                        .map_err(|e| format!("Failed to write claude config: {e}"))?;
                    Ok(assay_harness::claude::build_cli_args(&claude_config))
                },
            );
            assay_core::pipeline::run_manifest(&manifest, &pipeline_config, &harness_writer)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("pipeline task panicked: {e}"), None))?;

        // Build response
        let mut sessions = Vec::new();
        let mut succeeded = 0usize;
        let mut gate_failed = 0usize;
        let mut merge_conflict = 0usize;
        let mut errored = 0usize;

        for (i, result) in results.into_iter().enumerate() {
            let spec_name = session_specs[i].clone();
            match result {
                Ok(pr) => {
                    let timings: Vec<RunManifestStageTiming> = pr
                        .stage_timings
                        .iter()
                        .map(|t| RunManifestStageTiming {
                            stage: t.stage.to_string(),
                            duration_secs: t.duration.as_secs_f64(),
                        })
                        .collect();

                    match pr.outcome {
                        assay_core::pipeline::PipelineOutcome::Success => succeeded += 1,
                        assay_core::pipeline::PipelineOutcome::GateFailed => gate_failed += 1,
                        assay_core::pipeline::PipelineOutcome::MergeConflict => merge_conflict += 1,
                    }

                    sessions.push(RunManifestSessionResult {
                        spec_name,
                        session_id: Some(pr.session_id),
                        outcome: pr.outcome.to_string(),
                        error: None,
                        stage_timings: Some(timings),
                    });
                }
                Err(pe) => {
                    errored += 1;
                    sessions.push(RunManifestSessionResult {
                        spec_name,
                        session_id: None,
                        outcome: "Error".to_string(),
                        error: Some(RunManifestError {
                            stage: pe.stage.to_string(),
                            message: pe.message,
                            recovery: pe.recovery,
                            elapsed_secs: pe.elapsed.as_secs_f64(),
                        }),
                        stage_timings: None,
                    });
                }
            }
        }

        let total = sessions.len();
        let response = RunManifestResponse {
            sessions,
            summary: RunManifestSummary {
                total,
                succeeded,
                gate_failed,
                merge_conflict,
                errored,
            },
        };

        if errored > 0 {
            let json = serde_json::to_string(&response).map_err(|e| {
                McpError::internal_error(format!("serialization failed: {e}"), None)
            })?;
            Ok(CallToolResult::error(vec![Content::text(json)]))
        } else {
            let json = serde_json::to_string(&response).map_err(|e| {
                McpError::internal_error(format!("serialization failed: {e}"), None)
            })?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        }
    }

    /// Launch a multi-session orchestrated run with DAG-driven dispatch and post-execution merge.
    #[tool(
        description = "Run a multi-session manifest through the orchestrator: DAG-driven parallel dispatch → \
            per-session harness config → agent execution → base branch checkout → sequential merge. \
            Returns per-session outcomes, merge report, and a run_id for status queries via orchestrate_status. \
            Requires a manifest with multiple sessions or dependency edges. \
            The sync orchestration and merge phases are wrapped in spawn_blocking per D007."
    )]
    pub async fn orchestrate_run(
        &self,
        params: Parameters<OrchestrateRunParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };

        let manifest_path = PathBuf::from(&params.0.manifest_path);
        let manifest_path = if manifest_path.is_absolute() {
            manifest_path
        } else {
            cwd.join(&manifest_path)
        };

        // Load manifest (sync, cheap)
        let manifest = match assay_core::manifest::load(&manifest_path) {
            Ok(m) => m,
            Err(e) => return Ok(domain_error(&e)),
        };

        // Validate multi-session content (DAG mode only — Mesh/Gossip allow single sessions).
        let has_deps = manifest.sessions.iter().any(|s| !s.depends_on.is_empty());
        if manifest.mode == OrchestratorMode::Dag && manifest.sessions.len() < 2 && !has_deps {
            return Ok(CallToolResult::error(vec![Content::text(
                "Manifest must contain multiple sessions or dependency edges for orchestrated runs. \
                 Use run_manifest for single-session execution.",
            )]));
        }

        // Parse failure policy
        let failure_policy = match params.0.failure_policy.as_deref() {
            Some("abort") => assay_types::FailurePolicy::Abort,
            Some("skip_dependents") | None => assay_types::FailurePolicy::SkipDependents,
            Some(other) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid failure_policy '{other}'. Expected 'skip_dependents' or 'abort'.",
                ))]));
            }
        };

        // Parse merge strategy
        let merge_strategy = match params.0.merge_strategy.as_deref() {
            Some("file_overlap") => assay_types::MergeStrategy::FileOverlap,
            Some("completion_time") | None => assay_types::MergeStrategy::CompletionTime,
            Some(other) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid merge_strategy '{other}'. Expected 'completion_time' or 'file_overlap'.",
                ))]));
            }
        };

        // Parse conflict resolution mode
        let use_auto_conflict_resolution = match params.0.conflict_resolution.as_deref() {
            Some("auto") => true,
            Some("skip") | None => false,
            Some(other) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid conflict_resolution '{other}'. Expected 'auto' or 'skip'.",
                ))]));
            }
        };

        let orch_config = assay_core::orchestrate::executor::OrchestratorConfig {
            max_concurrency: 8,
            failure_policy,
        };

        let specs_dir = cwd.join(".assay").join(&config.specs_dir);
        let assay_dir = cwd.join(".assay");
        let worktree_base = assay_core::worktree::resolve_worktree_dir(None, &config, &cwd);
        let timeout_secs = params
            .0
            .timeout_secs
            .unwrap_or(assay_core::pipeline::PipelineConfig::DEFAULT_TIMEOUT_SECS);

        // Detect base branch: use git to find current branch before orchestration
        let base_branch = {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&cwd)
                .output()
                .map_err(|e| {
                    McpError::internal_error(format!("Failed to detect base branch: {e}"), None)
                })?;
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                "main".to_string()
            }
        };

        let pipeline_config = assay_core::pipeline::PipelineConfig {
            project_root: cwd.clone(),
            assay_dir: assay_dir.clone(),
            specs_dir,
            worktree_base,
            timeout_secs,
            base_branch: Some(base_branch.clone()),
        };

        // ── Mesh / Gossip stub routing ────────────────────────────────
        // These modes bypass the full DAG+merge pipeline and delegate to stubs.
        match manifest.mode {
            OrchestratorMode::Mesh => {
                let orch_config = assay_core::orchestrate::executor::OrchestratorConfig {
                    max_concurrency: 8,
                    failure_policy,
                };
                let manifest_clone = manifest.clone();
                let pipeline_config_clone = pipeline_config.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let session_runner = |_session: &assay_types::ManifestSession,
                                          _pipe_cfg: &assay_core::pipeline::PipelineConfig|
                     -> std::result::Result<
                        assay_core::pipeline::PipelineResult,
                        assay_core::pipeline::PipelineError,
                    > {
                        unreachable!("mesh stub does not invoke session_runner")
                    };
                    assay_core::orchestrate::mesh::run_mesh(
                        &manifest_clone,
                        &orch_config,
                        &pipeline_config_clone,
                        &session_runner,
                    )
                })
                .await
                .map_err(|e| McpError::internal_error(format!("mesh task panicked: {e}"), None))?;
                return match result {
                    Ok(orch_result) => {
                        let response = OrchestrateRunResponse {
                            run_id: orch_result.run_id,
                            duration_secs: orch_result.duration.as_secs_f64(),
                            failure_policy: format!("{:?}", orch_result.failure_policy),
                            sessions: vec![],
                            summary: OrchestrateRunSummary {
                                total: 0,
                                completed: 0,
                                failed: 0,
                                skipped: 0,
                            },
                            merge_report: None,
                        };
                        let json = serde_json::to_string(&response).map_err(|e| {
                            McpError::internal_error(format!("serialization failed: {e}"), None)
                        })?;
                        Ok(CallToolResult::success(vec![Content::text(json)]))
                    }
                    Err(e) => Ok(domain_error(&e)),
                };
            }
            OrchestratorMode::Gossip => {
                let orch_config = assay_core::orchestrate::executor::OrchestratorConfig {
                    max_concurrency: 8,
                    failure_policy,
                };
                let manifest_clone = manifest.clone();
                let pipeline_config_clone = pipeline_config.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let session_runner = |_session: &assay_types::ManifestSession,
                                          _pipe_cfg: &assay_core::pipeline::PipelineConfig|
                     -> std::result::Result<
                        assay_core::pipeline::PipelineResult,
                        assay_core::pipeline::PipelineError,
                    > {
                        unreachable!("gossip stub does not invoke session_runner")
                    };
                    assay_core::orchestrate::gossip::run_gossip(
                        &manifest_clone,
                        &orch_config,
                        &pipeline_config_clone,
                        &session_runner,
                    )
                })
                .await
                .map_err(|e| {
                    McpError::internal_error(format!("gossip task panicked: {e}"), None)
                })?;
                return match result {
                    Ok(orch_result) => {
                        let response = OrchestrateRunResponse {
                            run_id: orch_result.run_id,
                            duration_secs: orch_result.duration.as_secs_f64(),
                            failure_policy: format!("{:?}", orch_result.failure_policy),
                            sessions: vec![],
                            summary: OrchestrateRunSummary {
                                total: 0,
                                completed: 0,
                                failed: 0,
                                skipped: 0,
                            },
                            merge_report: None,
                        };
                        let json = serde_json::to_string(&response).map_err(|e| {
                            McpError::internal_error(format!("serialization failed: {e}"), None)
                        })?;
                        Ok(CallToolResult::success(vec![Content::text(json)]))
                    }
                    Err(e) => Ok(domain_error(&e)),
                };
            }
            OrchestratorMode::Dag => {} // fall through to existing DAG path
        }

        // Wrap the sync orchestration + merge in spawn_blocking (D007).
        let result = tokio::task::spawn_blocking(move || {
            // Session runner closure uses plain function calls (D035):
            // constructs HarnessWriter from generate_config/write_config/build_cli_args
            // rather than receiving a HarnessWriter dyn from the caller.
            let session_runner = |session: &assay_types::ManifestSession,
                                  pipe_cfg: &assay_core::pipeline::PipelineConfig|
             -> std::result::Result<
                assay_core::pipeline::PipelineResult,
                assay_core::pipeline::PipelineError,
            > {
                let harness_writer: Box<assay_core::pipeline::HarnessWriter> = Box::new(
                    |profile: &assay_types::HarnessProfile, worktree_path: &std::path::Path| {
                        let claude_config = assay_harness::claude::generate_config(profile);
                        assay_harness::claude::write_config(&claude_config, worktree_path)
                            .map_err(|e| format!("Failed to write claude config: {e}"))?;
                        Ok(assay_harness::claude::build_cli_args(&claude_config))
                    },
                );
                assay_core::pipeline::run_session(session, pipe_cfg, &harness_writer)
            };

            // Execute orchestration
            let orch_result = assay_core::orchestrate::executor::run_orchestrated(
                &manifest,
                orch_config,
                &pipeline_config,
                &session_runner,
            )?;

            // Checkout base branch before merge
            let project_root = &pipeline_config.project_root;
            let checkout_output = std::process::Command::new("git")
                .args(["checkout", &base_branch])
                .current_dir(project_root)
                .output()
                .map_err(|e| assay_core::AssayError::Io {
                    operation: format!("git checkout {}", base_branch),
                    path: project_root.clone(),
                    source: e,
                })?;
            if !checkout_output.status.success() {
                let stderr = String::from_utf8_lossy(&checkout_output.stderr);
                return Err(assay_core::AssayError::WorktreeGitFailed {
                    cmd: format!("git checkout {}", base_branch),
                    stderr: stderr.trim().to_string(),
                    exit_code: checkout_output.status.code(),
                });
            }

            // Extract completed sessions and merge
            let completed = assay_core::orchestrate::merge_runner::extract_completed_sessions(
                &orch_result.outcomes,
            );

            // Compose conflict handler based on resolution mode
            let merge_report = if use_auto_conflict_resolution {
                let cr_config = assay_types::orchestrate::ConflictResolutionConfig {
                    enabled: true,
                    ..Default::default()
                };
                let merge_config = assay_core::orchestrate::merge_runner::MergeRunnerConfig {
                    strategy: merge_strategy,
                    project_root: project_root.clone(),
                    base_branch: base_branch.clone(),
                    conflict_resolution_enabled: true,
                };
                let handler = move |name: &str,
                                    files: &[String],
                                    scan: &assay_types::ConflictScan,
                                    dir: &std::path::Path| {
                    assay_core::orchestrate::conflict_resolver::resolve_conflict(
                        name, files, scan, dir, &cr_config,
                    )
                };
                assay_core::orchestrate::merge_runner::merge_completed_sessions(
                    completed,
                    &merge_config,
                    handler,
                )?
            } else {
                let conflict_handler =
                    assay_core::orchestrate::merge_runner::default_conflict_handler();
                let merge_config = assay_core::orchestrate::merge_runner::MergeRunnerConfig {
                    strategy: merge_strategy,
                    project_root: project_root.clone(),
                    base_branch: base_branch.clone(),
                    conflict_resolution_enabled: false,
                };
                assay_core::orchestrate::merge_runner::merge_completed_sessions(
                    completed,
                    &merge_config,
                    conflict_handler,
                )?
            };

            // Persist merge report alongside state.json (non-fatal on failure).
            let run_dir = pipeline_config
                .project_root
                .join(".assay")
                .join("orchestrator")
                .join(&orch_result.run_id);
            if let Err(e) = persist_merge_report(&run_dir, &merge_report) {
                tracing::warn!(run_id = %orch_result.run_id, error = %e, "failed to persist merge report");
            }

            Ok::<_, assay_core::AssayError>((orch_result, merge_report))
        })
        .await
        .map_err(|e| McpError::internal_error(format!("orchestration task panicked: {e}"), None))?;

        match result {
            Ok((orch_result, merge_report)) => {
                let mut completed_count = 0usize;
                let mut failed_count = 0usize;
                let mut skipped_count = 0usize;
                let mut sessions = Vec::new();

                for (name, outcome) in &orch_result.outcomes {
                    match outcome {
                        assay_core::orchestrate::executor::SessionOutcome::Completed { .. } => {
                            completed_count += 1;
                            sessions.push(OrchestrateSessionOutcome {
                                name: name.clone(),
                                outcome: "completed".to_string(),
                                error: None,
                                skip_reason: None,
                            });
                        }
                        assay_core::orchestrate::executor::SessionOutcome::Failed {
                            error, ..
                        } => {
                            failed_count += 1;
                            sessions.push(OrchestrateSessionOutcome {
                                name: name.clone(),
                                outcome: "failed".to_string(),
                                error: Some(error.to_string()),
                                skip_reason: None,
                            });
                        }
                        assay_core::orchestrate::executor::SessionOutcome::Skipped { reason } => {
                            skipped_count += 1;
                            sessions.push(OrchestrateSessionOutcome {
                                name: name.clone(),
                                outcome: "skipped".to_string(),
                                error: None,
                                skip_reason: Some(reason.clone()),
                            });
                        }
                    }
                }

                let total = sessions.len();
                let response = OrchestrateRunResponse {
                    run_id: orch_result.run_id,
                    duration_secs: orch_result.duration.as_secs_f64(),
                    failure_policy: format!("{:?}", orch_result.failure_policy),
                    sessions,
                    summary: OrchestrateRunSummary {
                        total,
                        completed: completed_count,
                        failed: failed_count,
                        skipped: skipped_count,
                    },
                    merge_report: Some(merge_report),
                };

                let has_failures = failed_count > 0;
                let json = serde_json::to_string(&response).map_err(|e| {
                    McpError::internal_error(format!("serialization failed: {e}"), None)
                })?;

                if has_failures {
                    Ok(CallToolResult::error(vec![Content::text(json)]))
                } else {
                    Ok(CallToolResult::success(vec![Content::text(json)]))
                }
            }
            Err(e) => Ok(domain_error(&e)),
        }
    }

    /// Read persisted orchestrator state for a given run.
    #[tool(
        description = "Read the persisted orchestrator state for a given run ID. \
            Returns the full OrchestratorStatus snapshot including per-session states, phase, \
            and failure policy. Use this to inspect the state of a running or completed \
            orchestrated run launched by orchestrate_run."
    )]
    pub async fn orchestrate_status(
        &self,
        params: Parameters<OrchestrateStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let run_id = &params.0.run_id;
        let state_path = cwd
            .join(".assay")
            .join("orchestrator")
            .join(run_id)
            .join("state.json");

        let content = match std::fs::read_to_string(&state_path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "No orchestrator state found for run_id '{run_id}'. \
                     Check that the run_id is correct and that the run has started persisting state.",
                ))]));
            }
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to read orchestrator state for run_id '{run_id}': {e}",
                ))]));
            }
        };

        let status: assay_types::OrchestratorStatus = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to parse orchestrator state for run_id '{run_id}': {e}",
                ))]));
            }
        };

        // Try to load the merge report alongside state; non-fatal on missing/corrupt.
        let run_dir = cwd.join(".assay").join("orchestrator").join(run_id);
        let merge_report: Option<assay_types::MergeReport> = {
            let merge_report_path = run_dir.join("merge_report.json");
            match std::fs::read_to_string(&merge_report_path) {
                Ok(raw) => match serde_json::from_str(&raw) {
                    Ok(r) => Some(r),
                    Err(e) => {
                        tracing::warn!(run_id = %run_id, error = %e, "failed to parse merge report");
                        None
                    }
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => {
                    tracing::warn!(run_id = %run_id, error = %e, "failed to read merge report");
                    None
                }
            }
        };

        #[derive(Serialize)]
        struct OrchestrateStatusResponse {
            status: assay_types::OrchestratorStatus,
            merge_report: Option<assay_types::MergeReport>,
        }

        let response = OrchestrateStatusResponse {
            status,
            merge_report,
        };
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// List all milestones in the current project.
    #[tool(
        description = "List all milestones in the current project. Returns an array of milestone summaries including slug, name, status, and chunk count."
    )]
    pub async fn milestone_list(
        &self,
        _params: Parameters<MilestoneListParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let assay_dir = cwd.join(".assay");

        let milestones = match milestone_scan(&assay_dir) {
            Ok(m) => m,
            Err(e) => return Ok(domain_error(&e)),
        };

        let json = serde_json::to_string(&milestones)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get full details of a milestone by slug.
    #[tool(
        description = "Get full details of a milestone by slug, including all chunk references and status."
    )]
    pub async fn milestone_get(
        &self,
        params: Parameters<MilestoneGetParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let assay_dir = cwd.join(".assay");

        let milestone = match milestone_load(&assay_dir, &params.0.slug) {
            Ok(m) => m,
            Err(e) => return Ok(domain_error(&e)),
        };

        let json = serde_json::to_string(&milestone)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Return the active development cycle status.
    #[tool(
        description = "Return the active development cycle status: the first in_progress milestone, \
            its active chunk, and progress counts. Returns null if no milestone is in_progress."
    )]
    pub async fn cycle_status(
        &self,
        _params: Parameters<CycleStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let assay_dir = cwd.join(".assay");
        match assay_core::milestone::cycle_status(&assay_dir) {
            Ok(Some(status)) => {
                let json = serde_json::to_string(&status).map_err(|e| {
                    McpError::internal_error(format!("serialization failed: {e}"), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Ok(None) => Ok(CallToolResult::success(vec![Content::text("null")])),
            Err(e) => Ok(domain_error(&e)),
        }
    }

    /// Evaluate gates for the active chunk and advance the development cycle.
    #[tool(
        description = "Evaluate gates for the active chunk of the in_progress milestone and advance \
            the development cycle. Targets the first in_progress milestone unless milestone_slug is \
            specified. Returns updated CycleStatus on success, or error if required gates fail or \
            preconditions are not met."
    )]
    pub async fn cycle_advance(
        &self,
        params: Parameters<CycleAdvanceParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let config = match load_config(&cwd) {
            Ok(c) => c,
            Err(err_result) => return Ok(err_result),
        };
        let assay_dir = cwd.join(".assay");
        let specs_dir = cwd.join(".assay").join(&config.specs_dir);
        let working_dir = resolve_working_dir(&cwd, &config);
        let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);
        let milestone_slug = params.0.milestone_slug.clone();
        let result = tokio::task::spawn_blocking(move || {
            assay_core::milestone::cycle_advance(
                &assay_dir,
                &specs_dir,
                &working_dir,
                milestone_slug.as_deref(),
            )
        })
        .await
        .map_err(|e| McpError::internal_error(format!("cycle_advance panicked: {e}"), None))?;
        let _ = config_timeout; // used for doc consistency; core uses None/None for cli/config timeouts
        match result {
            Ok(status) => {
                let json = serde_json::to_string(&status).map_err(|e| {
                    McpError::internal_error(format!("serialization failed: {e}"), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(e) => Ok(domain_error(&e)),
        }
    }

    /// Return the last gate run result for a chunk without running new gates.
    #[tool(
        description = "Return the last gate run result for a chunk (spec) without running new gates. \
            Use this to check whether a chunk's gates passed in the most recent run. \
            Returns { has_history: false } when no run history exists for the chunk."
    )]
    pub async fn chunk_status(
        &self,
        params: Parameters<ChunkStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = resolve_cwd()?;
        let assay_dir = cwd.join(".assay");
        let chunk_slug = &params.0.chunk_slug;

        let all_ids = match assay_core::history::list(&assay_dir, chunk_slug) {
            Ok(ids) => ids,
            Err(e) => return Ok(domain_error(&e)),
        };

        if all_ids.is_empty() {
            let response = ChunkStatusResponse {
                chunk_slug: chunk_slug.clone(),
                has_history: false,
                latest_run_id: None,
                passed: None,
                failed: None,
                required_failed: None,
            };
            let json = serde_json::to_string(&response).map_err(|e| {
                McpError::internal_error(format!("serialization failed: {e}"), None)
            })?;
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        // list returns oldest-first; last() is the most recent run.
        let latest_run_id = all_ids.last().unwrap().clone();
        let record = match assay_core::history::load(&assay_dir, chunk_slug, &latest_run_id) {
            Ok(r) => r,
            Err(e) => return Ok(domain_error(&e)),
        };

        let response = ChunkStatusResponse {
            chunk_slug: chunk_slug.clone(),
            has_history: true,
            latest_run_id: Some(latest_run_id),
            passed: Some(record.summary.passed),
            failed: Some(record.summary.failed),
            required_failed: Some(record.summary.enforcement.required_failed),
        };
        let json = serde_json::to_string(&response)
            .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

// ── Private helpers ──────────────────────────────────────────────────

/// Atomically persist a `MergeReport` to `<run_dir>/merge_report.json`.
///
/// Uses a temp-file + rename pattern to avoid partial writes.
/// Returns an `io::Error` on failure; the caller logs a warning and continues.
fn persist_merge_report(run_dir: &Path, report: &assay_types::MergeReport) -> std::io::Result<()> {
    use std::io::Write as _;
    let final_path = run_dir.join("merge_report.json");
    let json = serde_json::to_string_pretty(report).map_err(std::io::Error::other)?;
    let mut tmpfile = tempfile::NamedTempFile::new_in(run_dir)?;
    tmpfile.write_all(json.as_bytes())?;
    tmpfile.as_file().sync_all()?;
    tmpfile.persist(&final_path).map_err(|e| e.error)?;
    Ok(())
}

impl AssayServer {
    /// Build a not-found error for a session ID, distinguishing timed-out
    /// sessions from sessions that never existed (or were already finalized).
    async fn session_not_found_error(&self, session_id: &str) -> CallToolResult {
        let timed_out = self.timed_out_sessions.lock().await;
        if let Some(info) = timed_out.get(session_id) {
            let elapsed = (info.timed_out_at - info.created_at).num_seconds();
            CallToolResult::error(vec![Content::text(format!(
                "Session '{session_id}' timed out after {elapsed}s \
                 (configured timeout: {}s) for spec '{}'. \
                 Use gate_run to start a new session, \
                 or gate_history to review past results.",
                info.timeout_secs, info.spec_name,
            ))])
        } else {
            CallToolResult::error(vec![Content::text(format!(
                "Session '{session_id}' not found \
                 (it may have been finalized or never existed). \
                 Use gate_run to start a new session, \
                 or gate_history to review past results.",
            ))])
        }
    }
}

#[tool_handler]
impl ServerHandler for AssayServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Assay development kit. Manages specs (what to build) and gates \
                 (quality checks). Use spec_list to discover specs, spec_get to \
                 read one, gate_run to evaluate criteria. For specs with agent \
                 criteria: use gate_evaluate for automated single-call evaluation \
                 via headless Claude Code subprocess, or gate_run + gate_report + \
                 gate_finalize for manual multi-step evaluation. \
                 Use gate_history to query past run results and track quality trends. \
                 Use context_diagnose for full session diagnostics with bloat analysis, \
                 or estimate_tokens for a quick context health check. \
                 Use worktree_create to isolate spec work in a git worktree, \
                 worktree_list/worktree_status to inspect, and worktree_cleanup to remove. \
                 Session tools track long-running work across the full agent lifecycle: \
                 session_create starts a persisted WorkSession tied to a spec and worktree \
                 (distinct from the ephemeral in-memory GateEvalContexts created automatically \
                 by gate_run for agent criteria). Use session_get to retrieve full session \
                 details by ID, session_update to advance phase and link gate run IDs, and \
                 session_list to enumerate sessions with optional filters. Choose gate_run \
                 when you need gate results immediately; choose session_create when you need \
                 to track an agent's work over time with phase transitions and linked gate runs."
                    .to_string(),
            ),
        }
    }
}

// ── Helper functions ─────────────────────────────────────────────────

/// Resolve the current working directory.
fn resolve_cwd() -> Result<PathBuf, McpError> {
    std::env::current_dir().map_err(|e| {
        McpError::internal_error(format!("cannot determine working directory: {e}"), None)
    })
}

/// Load and validate the Assay config from CWD.
fn load_config(cwd: &Path) -> Result<Config, CallToolResult> {
    assay_core::config::load(cwd).map_err(|e| domain_error(&e))
}

/// Load a spec entry by name, trying directory-based first, then legacy.
fn load_spec_entry_mcp(
    cwd: &Path,
    config: &Config,
    name: &str,
) -> Result<SpecEntry, CallToolResult> {
    let specs_dir = cwd.join(".assay").join(&config.specs_dir);
    assay_core::spec::load_spec_entry_with_diagnostics(name, &specs_dir)
        .map_err(|e| domain_error(&e))
}

/// Resolve the gate working directory from config, matching CLI behavior.
fn resolve_working_dir(cwd: &Path, config: &Config) -> PathBuf {
    match config.gates.as_ref().and_then(|g| g.working_dir.as_deref()) {
        Some(dir) => {
            let path = Path::new(dir);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            }
        }
        None => cwd.to_path_buf(),
    }
}

/// Convert an AssayError into a tool execution error that the agent can see and act on.
fn domain_error(err: &assay_core::AssayError) -> CallToolResult {
    CallToolResult::error(vec![Content::text(err.to_string())])
}

/// Truncate a string to at most `max_bytes`, respecting UTF-8 char boundaries.
fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Find the last char boundary at or before max_bytes
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Collected info about agent criteria in a spec, used to create a session.
struct AgentCriteriaInfo {
    agent_criteria_names: std::collections::HashSet<String>,
    spec_enforcement: HashMap<String, assay_types::Enforcement>,
}

/// A single criterion's metadata for agent criteria extraction.
type CriterionMeta<'a> = (
    &'a str,
    Option<CriterionKind>,
    Option<assay_types::Enforcement>,
);

/// Extract agent criteria information from a spec entry.
///
/// Returns `Some(info)` if the spec contains any `AgentReport` criteria,
/// `None` if all criteria are command-based or descriptive.
fn extract_agent_criteria_info(entry: &SpecEntry) -> Option<AgentCriteriaInfo> {
    let (criteria_iter, gate_section): (
        Box<dyn Iterator<Item = CriterionMeta<'_>>>,
        Option<&assay_types::GateSection>,
    ) = match entry {
        SpecEntry::Legacy { spec, .. } => (
            Box::new(
                spec.criteria
                    .iter()
                    .map(|c| (c.name.as_str(), c.kind, c.enforcement)),
            ),
            spec.gate.as_ref(),
        ),
        SpecEntry::Directory { gates, .. } => (
            Box::new(
                gates
                    .criteria
                    .iter()
                    .map(|c| (c.name.as_str(), c.kind, c.enforcement)),
            ),
            gates.gate.as_ref(),
        ),
    };

    let mut agent_criteria_names = std::collections::HashSet::new();
    let mut spec_enforcement = HashMap::new();

    for (name, kind, enforcement) in criteria_iter {
        if kind == Some(CriterionKind::AgentReport) {
            agent_criteria_names.insert(name.to_string());
        }
        let resolved = assay_core::gate::resolve_enforcement(enforcement, gate_section);
        spec_enforcement.insert(name.to_string(), resolved);
    }

    if agent_criteria_names.is_empty() {
        None
    } else {
        Some(AgentCriteriaInfo {
            agent_criteria_names,
            spec_enforcement,
        })
    }
}

/// Derive a human-readable kind label from a `GateKind`.
fn kind_label(kind: &assay_types::GateKind) -> Option<String> {
    match kind {
        assay_types::GateKind::Command { .. } => Some("cmd".to_string()),
        assay_types::GateKind::FileExists { .. } => Some("file".to_string()),
        assay_types::GateKind::AgentReport => Some("agent".to_string()),
        assay_types::GateKind::AlwaysPass => None,
    }
}

/// Map a `GateRunSummary` to the bounded `GateRunResponse` struct.
fn format_gate_response(
    summary: &assay_types::GateRunSummary,
    include_evidence: bool,
) -> GateRunResponse {
    let enforcement_label = |e: assay_types::Enforcement| -> String {
        match e {
            assay_types::Enforcement::Required => "required".to_string(),
            assay_types::Enforcement::Advisory => "advisory".to_string(),
        }
    };

    let criteria = summary
        .results
        .iter()
        .map(|cr| match &cr.result {
            None => CriterionSummary {
                name: cr.criterion_name.clone(),
                status: "skipped".to_string(),
                enforcement: enforcement_label(cr.enforcement),
                kind_label: None,
                exit_code: None,
                duration_ms: None,
                reason: None,
                stdout: None,
                stderr: None,
                truncated: None,
                original_bytes: None,
            },
            Some(gate_result) if gate_result.passed => CriterionSummary {
                name: cr.criterion_name.clone(),
                status: "passed".to_string(),
                enforcement: enforcement_label(cr.enforcement),
                kind_label: kind_label(&gate_result.kind),
                exit_code: gate_result.exit_code,
                duration_ms: Some(gate_result.duration_ms),
                reason: None,
                stdout: if include_evidence {
                    Some(gate_result.stdout.clone())
                } else {
                    None
                },
                stderr: if include_evidence {
                    Some(gate_result.stderr.clone())
                } else {
                    None
                },
                truncated: Some(gate_result.truncated),
                original_bytes: gate_result.original_bytes,
            },
            Some(gate_result) => {
                let reason = first_nonempty_line(&gate_result.stderr)
                    .or_else(|| first_nonempty_line(&gate_result.stdout))
                    .unwrap_or("unknown")
                    .to_string();
                CriterionSummary {
                    name: cr.criterion_name.clone(),
                    status: "failed".to_string(),
                    enforcement: enforcement_label(cr.enforcement),
                    kind_label: kind_label(&gate_result.kind),
                    exit_code: gate_result.exit_code,
                    duration_ms: Some(gate_result.duration_ms),
                    reason: Some(reason),
                    stdout: if include_evidence {
                        Some(gate_result.stdout.clone())
                    } else {
                        None
                    },
                    stderr: if include_evidence {
                        Some(gate_result.stderr.clone())
                    } else {
                        None
                    },
                    truncated: Some(gate_result.truncated),
                    original_bytes: gate_result.original_bytes,
                }
            }
        })
        .collect();

    GateRunResponse {
        spec_name: summary.spec_name.clone(),
        passed: summary.passed,
        failed: summary.failed,
        skipped: summary.skipped,
        required_passed: summary.enforcement.required_passed,
        required_failed: summary.enforcement.required_failed,
        advisory_passed: summary.enforcement.advisory_passed,
        advisory_failed: summary.enforcement.advisory_failed,
        blocked: summary.enforcement.required_failed > 0,
        total_duration_ms: summary.total_duration_ms,
        criteria,
        session_id: None,
        pending_criteria: None,
        warnings: Vec::new(),
    }
}

/// Extract the first non-empty line from a string, or `None` if all lines are empty.
fn first_nonempty_line(s: &str) -> Option<&str> {
    s.lines().find(|line| !line.trim().is_empty())
}

/// Load the stale session threshold from config, falling back to 3600 seconds.
fn load_recovery_threshold(cwd: &Path) -> u64 {
    let config_path = cwd.join(".assay").join("config.toml");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return 3600,
        Err(e) => {
            tracing::warn!("recovery threshold: cannot read config: {e}, using default 3600s");
            return 3600;
        }
    };
    let config: Config = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("recovery threshold: cannot parse config: {e}, using default 3600s");
            return 3600;
        }
    };
    config
        .sessions
        .map(|s| s.stale_threshold_secs)
        .unwrap_or(3600)
}

/// Start the MCP server on stdio transport.
///
/// Creates an [`AssayServer`] and serves JSON-RPC on stdin/stdout until
/// the transport closes. Caller must initialize tracing before calling.
///
/// Before accepting any tool calls, runs a recovery scan for stale
/// `agent_running` sessions (see [`assay_core::work_session::recover_stale_sessions`]).
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting assay MCP server");

    // Recover stale sessions before accepting any tool calls.
    // Runs synchronously before the server accepts connections. Capped at 100 sessions.
    if let Ok(cwd) = std::env::current_dir() {
        let assay_dir = cwd.join(".assay");
        if assay_dir.join("sessions").is_dir() {
            let stale_threshold = load_recovery_threshold(&cwd);
            // Recovery scan logs its own summary via tracing::info! when sessions are recovered.
            assay_core::work_session::recover_stale_sessions(&assay_dir, stale_threshold);
        }
    }

    let service = AssayServer::new().serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{
        CriterionResult, Enforcement, EnforcementSummary, GateKind, GateResult, GateRunSummary,
    };
    use chrono::Utc;
    use serial_test::serial;
    use std::io::Write as _;

    fn sample_summary() -> GateRunSummary {
        GateRunSummary {
            spec_name: "test-spec".to_string(),
            results: vec![
                CriterionResult {
                    criterion_name: "unit-tests".to_string(),
                    result: Some(GateResult {
                        passed: true,
                        kind: GateKind::Command {
                            cmd: "cargo test".to_string(),
                        },
                        stdout: "running 5 tests\ntest ok\n".to_string(),
                        stderr: String::new(),
                        exit_code: Some(0),
                        duration_ms: 1200,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Required,
                },
                CriterionResult {
                    criterion_name: "lint".to_string(),
                    result: Some(GateResult {
                        passed: false,
                        kind: GateKind::Command {
                            cmd: "cargo clippy".to_string(),
                        },
                        stdout: String::new(),
                        stderr: "error: unused variable\n  --> src/main.rs:5:9\n".to_string(),
                        exit_code: Some(1),
                        duration_ms: 800,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Required,
                },
                CriterionResult {
                    criterion_name: "review-checklist".to_string(),
                    result: None,
                    enforcement: Enforcement::Required,
                },
            ],
            passed: 1,
            failed: 1,
            skipped: 1,
            total_duration_ms: 2000,
            enforcement: EnforcementSummary {
                required_passed: 1,
                required_failed: 1,
                advisory_passed: 0,
                advisory_failed: 0,
            },
        }
    }

    #[test]
    fn test_format_gate_response_summary_mode() {
        let summary = sample_summary();
        let response = format_gate_response(&summary, false);

        assert_eq!(response.spec_name, "test-spec");
        assert_eq!(response.passed, 1);
        assert_eq!(response.failed, 1);
        assert_eq!(response.skipped, 1);
        assert_eq!(response.total_duration_ms, 2000);
        assert_eq!(response.required_passed, 1);
        assert_eq!(response.required_failed, 1);
        assert_eq!(response.advisory_passed, 0);
        assert_eq!(response.advisory_failed, 0);
        assert!(
            response.blocked,
            "blocked should be true when required_failed > 0"
        );
        assert_eq!(response.criteria.len(), 3);
        assert!(
            response.session_id.is_none(),
            "no session for cmd-only spec"
        );
        assert!(response.pending_criteria.is_none());

        // Passed criterion
        let passed = &response.criteria[0];
        assert_eq!(passed.name, "unit-tests");
        assert_eq!(passed.status, "passed");
        assert_eq!(passed.kind_label.as_deref(), Some("cmd"));
        assert_eq!(passed.exit_code, Some(0));
        assert_eq!(passed.duration_ms, Some(1200));
        assert!(
            passed.reason.is_none(),
            "passed criteria should not have reason"
        );
        assert!(
            passed.stdout.is_none(),
            "summary mode should not have stdout"
        );
        assert!(
            passed.stderr.is_none(),
            "summary mode should not have stderr"
        );

        // Failed criterion
        let failed = &response.criteria[1];
        assert_eq!(failed.name, "lint");
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.kind_label.as_deref(), Some("cmd"));
        assert_eq!(failed.exit_code, Some(1));
        assert_eq!(failed.duration_ms, Some(800));
        assert!(
            failed
                .reason
                .as_deref()
                .unwrap()
                .contains("unused variable"),
            "failed reason should contain first stderr line, got: {:?}",
            failed.reason
        );
        assert!(
            failed.stdout.is_none(),
            "summary mode should not have stdout"
        );
        assert!(
            failed.stderr.is_none(),
            "summary mode should not have stderr"
        );

        // Skipped criterion
        let skipped = &response.criteria[2];
        assert_eq!(skipped.name, "review-checklist");
        assert_eq!(skipped.status, "skipped");
        assert!(skipped.kind_label.is_none(), "skipped has no kind_label");
        assert!(skipped.exit_code.is_none());
        assert!(skipped.duration_ms.is_none());
        assert!(skipped.reason.is_none());
        assert!(skipped.stdout.is_none());
        assert!(skipped.stderr.is_none());
    }

    #[test]
    fn test_format_gate_response_evidence_mode() {
        let summary = sample_summary();
        let response = format_gate_response(&summary, true);

        // Passed criterion should include stdout/stderr
        let passed = &response.criteria[0];
        assert!(
            passed.stdout.is_some(),
            "evidence mode should include stdout for passed"
        );
        assert!(
            passed.stderr.is_some(),
            "evidence mode should include stderr for passed"
        );
        assert!(
            passed
                .stdout
                .as_deref()
                .unwrap()
                .contains("running 5 tests"),
            "stdout should contain actual output"
        );

        // Failed criterion should include stdout/stderr
        let failed = &response.criteria[1];
        assert!(
            failed.stdout.is_some(),
            "evidence mode should include stdout for failed"
        );
        assert!(
            failed.stderr.is_some(),
            "evidence mode should include stderr for failed"
        );
        assert!(
            failed
                .stderr
                .as_deref()
                .unwrap()
                .contains("unused variable"),
            "stderr should contain actual output"
        );

        // Skipped criterion still has no evidence
        let skipped = &response.criteria[2];
        assert!(skipped.stdout.is_none());
        assert!(skipped.stderr.is_none());
    }

    #[test]
    fn test_domain_error_produces_error_result() {
        let err = assay_core::AssayError::SpecNotFound {
            name: "auth-flow".to_string(),
            specs_dir: std::path::PathBuf::from(".assay/specs/"),
        };

        let result = domain_error(&err);

        // CallToolResult should be an error (is_error: true)
        assert!(
            result.is_error.unwrap_or(false),
            "domain_error should produce isError: true"
        );

        // The content should contain the error message
        let text = result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        assert!(
            text.contains("auth-flow"),
            "error text should contain spec name, got: {text}"
        );
        assert!(
            text.contains(".assay/specs/"),
            "error text should contain specs dir, got: {text}"
        );
    }

    #[test]
    fn test_first_nonempty_line() {
        assert_eq!(first_nonempty_line("hello\nworld"), Some("hello"));
        assert_eq!(first_nonempty_line("\n\nhello"), Some("hello"));
        assert_eq!(first_nonempty_line("  \n  \n"), None);
        assert_eq!(first_nonempty_line(""), None);
        assert_eq!(
            first_nonempty_line("error: unused variable\n  --> src/main.rs"),
            Some("error: unused variable")
        );
    }

    #[test]
    fn test_format_gate_response_failed_with_empty_stderr() {
        let summary = GateRunSummary {
            spec_name: "test".to_string(),
            results: vec![CriterionResult {
                criterion_name: "silent-fail".to_string(),
                result: Some(GateResult {
                    passed: false,
                    kind: GateKind::Command {
                        cmd: "false".to_string(),
                    },
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: Some(1),
                    duration_ms: 50,
                    timestamp: Utc::now(),
                    truncated: false,
                    original_bytes: None,
                    evidence: None,
                    reasoning: None,
                    confidence: None,
                    evaluator_role: None,
                }),
                enforcement: Enforcement::Required,
            }],
            passed: 0,
            failed: 1,
            skipped: 0,
            total_duration_ms: 50,
            enforcement: EnforcementSummary {
                required_passed: 0,
                required_failed: 1,
                advisory_passed: 0,
                advisory_failed: 0,
            },
        };

        let response = format_gate_response(&summary, false);
        let failed = &response.criteria[0];
        assert_eq!(
            failed.reason.as_deref(),
            Some("unknown"),
            "failed with empty stderr should have 'unknown' reason"
        );
        assert!(
            response.blocked,
            "blocked should be true when required_failed > 0"
        );
    }

    #[test]
    fn test_failure_reason_prefers_stderr() {
        let summary = GateRunSummary {
            spec_name: "test".to_string(),
            results: vec![CriterionResult {
                criterion_name: "both-outputs".to_string(),
                result: Some(GateResult {
                    passed: false,
                    kind: GateKind::Command {
                        cmd: "failing-cmd".to_string(),
                    },
                    stdout: "stdout message".to_string(),
                    stderr: "stderr message".to_string(),
                    exit_code: Some(1),
                    duration_ms: 42,
                    timestamp: Utc::now(),
                    truncated: false,
                    original_bytes: None,
                    evidence: None,
                    reasoning: None,
                    confidence: None,
                    evaluator_role: None,
                }),
                enforcement: Enforcement::Required,
            }],
            passed: 0,
            failed: 1,
            skipped: 0,
            total_duration_ms: 42,
            enforcement: EnforcementSummary {
                required_passed: 0,
                required_failed: 1,
                advisory_passed: 0,
                advisory_failed: 0,
            },
        };

        let response = format_gate_response(&summary, false);
        let failed = &response.criteria[0];
        assert_eq!(
            failed.reason.as_deref(),
            Some("stderr message"),
            "when both stderr and stdout are present, reason should come from stderr"
        );
    }

    #[test]
    fn test_failure_reason_falls_back_to_stdout() {
        let summary = GateRunSummary {
            spec_name: "test".to_string(),
            results: vec![CriterionResult {
                criterion_name: "stdout-only".to_string(),
                result: Some(GateResult {
                    passed: false,
                    kind: GateKind::Command {
                        cmd: "failing-cmd".to_string(),
                    },
                    stdout: "error from stdout".to_string(),
                    stderr: String::new(),
                    exit_code: Some(1),
                    duration_ms: 30,
                    timestamp: Utc::now(),
                    truncated: false,
                    original_bytes: None,
                    evidence: None,
                    reasoning: None,
                    confidence: None,
                    evaluator_role: None,
                }),
                enforcement: Enforcement::Required,
            }],
            passed: 0,
            failed: 1,
            skipped: 0,
            total_duration_ms: 30,
            enforcement: EnforcementSummary {
                required_passed: 0,
                required_failed: 1,
                advisory_passed: 0,
                advisory_failed: 0,
            },
        };

        let response = format_gate_response(&summary, false);
        let failed = &response.criteria[0];
        assert_eq!(
            failed.reason.as_deref(),
            Some("error from stdout"),
            "when stderr is empty, reason should fall back to stdout"
        );
    }

    // Note: both-empty → "unknown" case already covered by
    // test_format_gate_response_failed_with_empty_stderr above.

    /// Build a `GateRunSummary` with a single failing criterion, reducing boilerplate in
    /// failure-reason tests.
    fn single_failing_summary(criterion_name: &str, stdout: &str, stderr: &str) -> GateRunSummary {
        GateRunSummary {
            spec_name: "test".to_string(),
            results: vec![CriterionResult {
                criterion_name: criterion_name.to_string(),
                result: Some(GateResult {
                    passed: false,
                    kind: GateKind::Command {
                        cmd: "failing-cmd".to_string(),
                    },
                    stdout: stdout.to_string(),
                    stderr: stderr.to_string(),
                    exit_code: Some(1),
                    duration_ms: 42,
                    timestamp: Utc::now(),
                    truncated: false,
                    original_bytes: None,
                    evidence: None,
                    reasoning: None,
                    confidence: None,
                    evaluator_role: None,
                }),
                enforcement: Enforcement::Required,
            }],
            passed: 0,
            failed: 1,
            skipped: 0,
            total_duration_ms: 42,
            enforcement: EnforcementSummary {
                required_passed: 0,
                required_failed: 1,
                advisory_passed: 0,
                advisory_failed: 0,
            },
        }
    }

    #[test]
    fn test_failure_reason_stdout_multiline_uses_first_nonempty_line() {
        // When stdout has multiple lines, only the first non-empty line should be the reason.
        let summary = single_failing_summary("multiline-out", "first line\nsecond line\n", "");
        let response = format_gate_response(&summary, false);
        assert_eq!(
            response.criteria[0].reason.as_deref(),
            Some("first line"),
            "reason should be first non-empty line of stdout, not entire output"
        );
    }

    #[test]
    fn test_failure_reason_stdout_skips_leading_empty_lines() {
        // Leading empty lines in stdout should be skipped; the first non-empty line is used.
        let summary = single_failing_summary("leading-empty", "\n\nreal error\nmore stuff\n", "");
        let response = format_gate_response(&summary, false);
        assert_eq!(
            response.criteria[0].reason.as_deref(),
            Some("real error"),
            "reason should skip leading empty lines and return first non-empty line"
        );
    }

    // ── Helper function integration tests ────────────────────────────

    /// Create a tempdir with a valid `.assay/config.toml`.
    fn create_project(config_toml: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();
        let mut f = std::fs::File::create(assay_dir.join("config.toml")).unwrap();
        f.write_all(config_toml.as_bytes()).unwrap();
        dir
    }

    /// Create a spec file inside a project's specs directory.
    fn create_spec(project_dir: &Path, specs_dir_name: &str, filename: &str, content: &str) {
        let specs_path = project_dir.join(".assay").join(specs_dir_name);
        std::fs::create_dir_all(&specs_path).unwrap();
        let mut f = std::fs::File::create(specs_path.join(filename)).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_load_config_valid_project() {
        let dir = create_project(r#"project_name = "test-project""#);
        let config = load_config(dir.path());

        assert!(
            config.is_ok(),
            "load_config should succeed for valid project, got: {:?}",
            config.err()
        );
        let config = config.unwrap();
        assert_eq!(config.project_name, "test-project");
    }

    #[test]
    fn test_load_config_missing_project() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_config(dir.path());

        assert!(
            result.is_err(),
            "load_config should fail for missing .assay/"
        );
        let err_result = result.unwrap_err();
        assert!(
            err_result.is_error.unwrap_or(false),
            "should produce isError: true"
        );

        // Check that error text mentions the path
        let text: String = err_result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            text.contains("config"),
            "error should mention config, got: {text}"
        );
    }

    #[test]
    fn test_load_spec_entry_not_found() {
        let dir = create_project(r#"project_name = "test""#);
        // Create the specs directory but no spec files
        std::fs::create_dir_all(dir.path().join(".assay").join("specs")).unwrap();

        let config = load_config(dir.path()).unwrap();
        let result = load_spec_entry_mcp(dir.path(), &config, "nonexistent");

        assert!(
            result.is_err(),
            "load_spec_entry_mcp should fail for nonexistent spec"
        );
        let err_result = result.unwrap_err();
        assert!(
            err_result.is_error.unwrap_or(false),
            "should produce isError: true"
        );

        let text: String = err_result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            text.contains("No specs found"),
            "error should contain 'No specs found', got: {text}"
        );
    }

    #[test]
    fn test_load_spec_entry_legacy() {
        let dir = create_project(r#"project_name = "test""#);
        create_spec(
            dir.path(),
            "specs",
            "auth-flow.toml",
            r#"
name = "auth-flow"
description = "Auth spec"

[[criteria]]
name = "unit-tests"
description = "Tests pass"
cmd = "echo ok"
"#,
        );

        let config = load_config(dir.path()).unwrap();
        let result = load_spec_entry_mcp(dir.path(), &config, "auth-flow");

        assert!(
            result.is_ok(),
            "load_spec_entry_mcp should succeed for valid spec, got: {:?}",
            result.err()
        );
        let entry = result.unwrap();
        assert!(matches!(entry, SpecEntry::Legacy { .. }));
        assert_eq!(entry.name(), "auth-flow");
    }

    #[test]
    fn test_load_spec_entry_directory() {
        let dir = create_project(r#"project_name = "test""#);
        let specs_dir = dir.path().join(".assay").join("specs").join("auth-dir");
        std::fs::create_dir_all(&specs_dir).unwrap();
        std::fs::write(
            specs_dir.join("gates.toml"),
            r#"
name = "auth-dir"

[[criteria]]
name = "compiles"
description = "Code compiles"
cmd = "echo ok"
"#,
        )
        .unwrap();

        let config = load_config(dir.path()).unwrap();
        let result = load_spec_entry_mcp(dir.path(), &config, "auth-dir");

        assert!(result.is_ok());
        let entry = result.unwrap();
        assert!(matches!(entry, SpecEntry::Directory { .. }));
        assert_eq!(entry.name(), "auth-dir");
    }

    #[test]
    fn test_resolve_working_dir_default() {
        let cwd = PathBuf::from("/some/project");
        let config = assay_types::Config {
            project_name: "test".to_string(),
            specs_dir: "specs/".to_string(),
            gates: None,
            guard: None,
            worktree: None,
            sessions: None,
        };

        let result = resolve_working_dir(&cwd, &config);
        assert_eq!(result, cwd, "default should return cwd");
    }

    #[test]
    fn test_resolve_working_dir_relative() {
        let cwd = PathBuf::from("/some/project");
        let config = assay_types::Config {
            project_name: "test".to_string(),
            specs_dir: "specs/".to_string(),
            gates: Some(assay_types::GatesConfig {
                default_timeout: 300,
                working_dir: Some("subdir".to_string()),
                max_history: None,
                evaluator_model: "sonnet".to_string(),
                evaluator_retries: 1,
                evaluator_timeout: 120,
            }),
            guard: None,
            worktree: None,
            sessions: None,
        };

        let result = resolve_working_dir(&cwd, &config);
        assert_eq!(
            result,
            PathBuf::from("/some/project/subdir"),
            "relative dir should be joined to cwd"
        );
    }

    #[test]
    fn test_resolve_working_dir_absolute() {
        let cwd = PathBuf::from("/some/project");
        let config = assay_types::Config {
            project_name: "test".to_string(),
            specs_dir: "specs/".to_string(),
            gates: Some(assay_types::GatesConfig {
                default_timeout: 300,
                working_dir: Some("/tmp/custom".to_string()),
                max_history: None,
                evaluator_model: "sonnet".to_string(),
                evaluator_retries: 1,
                evaluator_timeout: 120,
            }),
            guard: None,
            worktree: None,
            sessions: None,
        };

        let result = resolve_working_dir(&cwd, &config);
        assert_eq!(
            result,
            PathBuf::from("/tmp/custom"),
            "absolute dir should be used as-is"
        );
    }

    // ── Response serialization tests ─────────────────────────────────

    #[test]
    fn test_spec_list_entry_serialization() {
        let entry = SpecListEntry {
            name: "auth-flow".to_string(),
            description: "Authentication flow".to_string(),
            criteria_count: 3,
            format: "legacy".to_string(),
            has_feature_spec: false,
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["name"], "auth-flow");
        assert_eq!(json["description"], "Authentication flow");
        assert_eq!(json["criteria_count"], 3);
        assert_eq!(json["format"], "legacy");
        assert!(
            json.get("has_feature_spec").is_none(),
            "false has_feature_spec should be omitted"
        );
    }

    #[test]
    fn test_spec_list_entry_omits_empty_description() {
        let entry = SpecListEntry {
            name: "bare-spec".to_string(),
            description: String::new(),
            criteria_count: 1,
            format: "legacy".to_string(),
            has_feature_spec: false,
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["name"], "bare-spec");
        assert!(
            json.get("description").is_none(),
            "empty description should be omitted, got: {json}"
        );
        assert_eq!(json["criteria_count"], 1);
    }

    #[test]
    fn test_spec_list_entry_directory_format() {
        let entry = SpecListEntry {
            name: "auth-dir".to_string(),
            description: String::new(),
            criteria_count: 2,
            format: "directory".to_string(),
            has_feature_spec: true,
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["format"], "directory");
        assert_eq!(json["has_feature_spec"], true);
    }

    #[test]
    fn test_gate_run_response_serialization() {
        let response = GateRunResponse {
            spec_name: "auth-flow".to_string(),
            passed: 2,
            failed: 1,
            skipped: 1,
            required_passed: 1,
            required_failed: 1,
            advisory_passed: 0,
            advisory_failed: 0,
            blocked: true,
            total_duration_ms: 1500,
            criteria: vec![
                CriterionSummary {
                    name: "unit-tests".to_string(),
                    status: "passed".to_string(),
                    enforcement: "required".to_string(),
                    kind_label: Some("cmd".to_string()),
                    exit_code: Some(0),
                    duration_ms: Some(800),
                    reason: None,
                    stdout: None,
                    stderr: None,
                    truncated: Some(false),
                    original_bytes: None,
                },
                CriterionSummary {
                    name: "lint".to_string(),
                    status: "failed".to_string(),
                    enforcement: "required".to_string(),
                    kind_label: Some("cmd".to_string()),
                    exit_code: Some(1),
                    duration_ms: Some(700),
                    reason: Some("error: unused variable".to_string()),
                    stdout: None,
                    stderr: None,
                    truncated: Some(false),
                    original_bytes: None,
                },
                CriterionSummary {
                    name: "review".to_string(),
                    status: "skipped".to_string(),
                    enforcement: "required".to_string(),
                    kind_label: None,
                    exit_code: None,
                    duration_ms: None,
                    reason: None,
                    stdout: None,
                    stderr: None,
                    truncated: None,
                    original_bytes: None,
                },
            ],
            session_id: None,
            pending_criteria: None,
            warnings: Vec::new(),
        };

        let json = serde_json::to_value(&response).unwrap();

        // Top-level fields
        assert_eq!(json["spec_name"], "auth-flow");
        assert_eq!(json["passed"], 2);
        assert_eq!(json["failed"], 1);
        assert_eq!(json["skipped"], 1);
        assert_eq!(json["required_passed"], 1);
        assert_eq!(json["required_failed"], 1);
        assert_eq!(json["advisory_passed"], 0);
        assert_eq!(json["advisory_failed"], 0);
        assert_eq!(json["blocked"], true);
        assert_eq!(json["total_duration_ms"], 1500);

        // No session fields when no agent criteria
        assert!(
            json.get("session_id").is_none(),
            "session_id should be omitted: {json}"
        );
        assert!(
            json.get("pending_criteria").is_none(),
            "pending_criteria should be omitted: {json}"
        );

        // Passed criterion: has kind_label, no reason, no stdout, no stderr in JSON
        let passed = &json["criteria"][0];
        assert_eq!(passed["name"], "unit-tests");
        assert_eq!(passed["status"], "passed");
        assert_eq!(passed["kind_label"], "cmd");
        assert_eq!(passed["exit_code"], 0);
        assert!(
            passed.get("reason").is_none(),
            "passed should not have reason: {passed}"
        );
        assert!(
            passed.get("stdout").is_none(),
            "summary mode should not have stdout: {passed}"
        );
        assert!(
            passed.get("stderr").is_none(),
            "summary mode should not have stderr: {passed}"
        );

        // Failed criterion: has reason, no stdout/stderr
        let failed = &json["criteria"][1];
        assert_eq!(failed["name"], "lint");
        assert_eq!(failed["reason"], "error: unused variable");
        assert!(
            failed.get("stdout").is_none(),
            "summary mode should not have stdout: {failed}"
        );

        // Skipped criterion: minimal fields only, no kind_label
        let skipped = &json["criteria"][2];
        assert_eq!(skipped["name"], "review");
        assert_eq!(skipped["status"], "skipped");
        assert!(
            skipped.get("kind_label").is_none(),
            "skipped should not have kind_label: {skipped}"
        );
        assert!(
            skipped.get("exit_code").is_none(),
            "skipped should not have exit_code: {skipped}"
        );
        assert!(
            skipped.get("duration_ms").is_none(),
            "skipped should not have duration_ms: {skipped}"
        );
        assert!(
            skipped.get("reason").is_none(),
            "skipped should not have reason: {skipped}"
        );
    }

    #[test]
    fn test_gate_run_response_with_evidence() {
        let response = GateRunResponse {
            spec_name: "test".to_string(),
            passed: 1,
            failed: 0,
            skipped: 0,
            required_passed: 1,
            required_failed: 0,
            advisory_passed: 0,
            advisory_failed: 0,
            blocked: false,
            total_duration_ms: 500,
            criteria: vec![CriterionSummary {
                name: "check".to_string(),
                status: "passed".to_string(),
                enforcement: "required".to_string(),
                kind_label: Some("cmd".to_string()),
                exit_code: Some(0),
                duration_ms: Some(500),
                reason: None,
                stdout: Some("all tests passed".to_string()),
                stderr: Some(String::new()),
                truncated: Some(false),
                original_bytes: None,
            }],
            session_id: None,
            pending_criteria: None,
            warnings: Vec::new(),
        };

        let json = serde_json::to_value(&response).unwrap();
        let criterion = &json["criteria"][0];

        assert_eq!(criterion["stdout"], "all tests passed");
        assert!(
            criterion.get("stderr").is_some(),
            "evidence mode should include stderr even when empty"
        );
    }

    // ── New tool response serialization tests ───────────────────────

    #[test]
    fn test_gate_report_response_serialization() {
        let response = GateReportResponse {
            session_id: "20260305T220000Z-abc123".to_string(),
            criterion_name: "code-review".to_string(),
            accepted: true,
            evaluations_count: 1,
            pending_criteria: vec!["arch-review".to_string()],
            warnings: Vec::new(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["session_id"], "20260305T220000Z-abc123");
        assert_eq!(json["criterion_name"], "code-review");
        assert_eq!(json["accepted"], true);
        assert_eq!(json["evaluations_count"], 1);
        assert_eq!(json["pending_criteria"][0], "arch-review");
    }

    #[test]
    fn test_gate_run_response_with_session() {
        let response = GateRunResponse {
            spec_name: "mixed-spec".to_string(),
            passed: 1,
            failed: 0,
            skipped: 1,
            required_passed: 1,
            required_failed: 0,
            advisory_passed: 0,
            advisory_failed: 0,
            blocked: false,
            total_duration_ms: 500,
            criteria: vec![],
            session_id: Some("20260305T220000Z-abc123".to_string()),
            pending_criteria: Some(vec!["code-review".to_string()]),
            warnings: Vec::new(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["session_id"], "20260305T220000Z-abc123");
        assert_eq!(json["pending_criteria"][0], "code-review");
    }

    #[test]
    fn test_kind_label_command() {
        assert_eq!(
            kind_label(&GateKind::Command {
                cmd: "echo ok".to_string()
            }),
            Some("cmd".to_string())
        );
    }

    #[test]
    fn test_kind_label_file_exists() {
        assert_eq!(
            kind_label(&GateKind::FileExists {
                path: "README.md".to_string()
            }),
            Some("file".to_string())
        );
    }

    #[test]
    fn test_kind_label_agent_report() {
        assert_eq!(
            kind_label(&GateKind::AgentReport),
            Some("agent".to_string())
        );
    }

    #[test]
    fn test_kind_label_always_pass() {
        assert_eq!(kind_label(&GateKind::AlwaysPass), None);
    }

    #[test]
    fn test_extract_agent_criteria_none_for_cmd_only() {
        let entry = SpecEntry::Legacy {
            slug: "test".to_string(),
            spec: assay_types::Spec {
                name: "test".to_string(),
                description: String::new(),
                gate: None,
                depends: vec![],
                criteria: vec![assay_types::Criterion {
                    name: "builds".to_string(),
                    description: "builds".to_string(),
                    cmd: Some("cargo build".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                }],
            },
        };

        assert!(
            extract_agent_criteria_info(&entry).is_none(),
            "cmd-only spec should not have agent criteria"
        );
    }

    #[test]
    fn test_extract_agent_criteria_some_for_mixed_spec() {
        let entry = SpecEntry::Legacy {
            slug: "mixed".to_string(),
            spec: assay_types::Spec {
                name: "mixed".to_string(),
                description: String::new(),
                gate: None,
                depends: vec![],
                criteria: vec![
                    assay_types::Criterion {
                        name: "builds".to_string(),
                        description: "builds".to_string(),
                        cmd: Some("cargo build".to_string()),
                        path: None,
                        timeout: None,
                        enforcement: None,
                        kind: None,
                        prompt: None,
                        requirements: vec![],
                    },
                    assay_types::Criterion {
                        name: "code-review".to_string(),
                        description: "Agent reviews code".to_string(),
                        cmd: None,
                        path: None,
                        timeout: None,
                        enforcement: None,
                        kind: Some(CriterionKind::AgentReport),
                        prompt: Some("Review for issues".to_string()),
                        requirements: vec![],
                    },
                ],
            },
        };

        let info = extract_agent_criteria_info(&entry);
        assert!(info.is_some(), "mixed spec should have agent criteria");
        let info = info.unwrap();
        assert!(info.agent_criteria_names.contains("code-review"));
        assert!(!info.agent_criteria_names.contains("builds"));
        assert_eq!(info.spec_enforcement.len(), 2);
    }

    #[test]
    fn test_format_gate_response_agent_result_kind_label() {
        let summary = GateRunSummary {
            spec_name: "agent-test".to_string(),
            results: vec![CriterionResult {
                criterion_name: "review".to_string(),
                result: Some(GateResult {
                    passed: true,
                    kind: GateKind::AgentReport,
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    duration_ms: 0,
                    timestamp: Utc::now(),
                    truncated: false,
                    original_bytes: None,
                    evidence: Some("found auth module".to_string()),
                    reasoning: Some("auth module uses JWT".to_string()),
                    confidence: Some(assay_types::Confidence::High),
                    evaluator_role: Some(EvaluatorRole::SelfEval),
                }),
                enforcement: Enforcement::Advisory,
            }],
            passed: 1,
            failed: 0,
            skipped: 0,
            total_duration_ms: 0,
            enforcement: EnforcementSummary::default(),
        };

        let response = format_gate_response(&summary, false);
        let cr = &response.criteria[0];
        assert_eq!(cr.kind_label.as_deref(), Some("agent"));
        assert_eq!(cr.status, "passed");
        assert_eq!(cr.enforcement, "advisory");
    }

    #[test]
    fn test_spec_list_response_serialization_with_errors() {
        let response = SpecListResponse {
            specs: vec![SpecListEntry {
                name: "auth".to_string(),
                description: "Auth flow".to_string(),
                criteria_count: 2,
                format: "legacy".to_string(),
                has_feature_spec: false,
            }],
            errors: vec![SpecListError {
                message: "failed to parse broken.toml: invalid key".to_string(),
            }],
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["specs"][0]["name"], "auth");
        assert_eq!(
            json["errors"][0]["message"],
            "failed to parse broken.toml: invalid key"
        );
    }

    #[test]
    fn test_spec_list_response_serialization_without_errors() {
        let response = SpecListResponse {
            specs: vec![SpecListEntry {
                name: "clean".to_string(),
                description: String::new(),
                criteria_count: 1,
                format: "legacy".to_string(),
                has_feature_spec: false,
            }],
            errors: vec![],
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["specs"][0]["name"], "clean");
        assert!(
            json.get("errors").is_none(),
            "empty errors should be omitted via skip_serializing_if"
        );
    }

    #[test]
    fn test_format_gate_response_enforcement_counts() {
        let summary = GateRunSummary {
            spec_name: "enforcement-test".to_string(),
            results: vec![
                CriterionResult {
                    criterion_name: "req-pass".to_string(),
                    result: Some(GateResult {
                        passed: true,
                        kind: GateKind::Command {
                            cmd: "true".to_string(),
                        },
                        stdout: String::new(),
                        stderr: String::new(),
                        exit_code: Some(0),
                        duration_ms: 10,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Required,
                },
                CriterionResult {
                    criterion_name: "adv-pass".to_string(),
                    result: Some(GateResult {
                        passed: true,
                        kind: GateKind::Command {
                            cmd: "true".to_string(),
                        },
                        stdout: String::new(),
                        stderr: String::new(),
                        exit_code: Some(0),
                        duration_ms: 10,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Advisory,
                },
            ],
            passed: 2,
            failed: 0,
            skipped: 0,
            total_duration_ms: 20,
            enforcement: EnforcementSummary {
                required_passed: 1,
                required_failed: 0,
                advisory_passed: 1,
                advisory_failed: 0,
            },
        };

        let response = format_gate_response(&summary, false);
        assert_eq!(response.required_passed, 1);
        assert_eq!(response.required_failed, 0);
        assert_eq!(response.advisory_passed, 1);
        assert_eq!(response.advisory_failed, 0);
        assert!(
            !response.blocked,
            "blocked should be false when no required failures"
        );
    }

    #[test]
    fn test_format_gate_response_advisory_failed_not_blocked() {
        let summary = GateRunSummary {
            spec_name: "advisory-fail".to_string(),
            results: vec![
                CriterionResult {
                    criterion_name: "req-pass".to_string(),
                    result: Some(GateResult {
                        passed: true,
                        kind: GateKind::Command {
                            cmd: "true".to_string(),
                        },
                        stdout: String::new(),
                        stderr: String::new(),
                        exit_code: Some(0),
                        duration_ms: 10,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Required,
                },
                CriterionResult {
                    criterion_name: "adv-fail".to_string(),
                    result: Some(GateResult {
                        passed: false,
                        kind: GateKind::Command {
                            cmd: "false".to_string(),
                        },
                        stdout: String::new(),
                        stderr: "lint warning".to_string(),
                        exit_code: Some(1),
                        duration_ms: 10,
                        timestamp: Utc::now(),
                        truncated: false,
                        original_bytes: None,
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Advisory,
                },
            ],
            passed: 1,
            failed: 1,
            skipped: 0,
            total_duration_ms: 20,
            enforcement: EnforcementSummary {
                required_passed: 1,
                required_failed: 0,
                advisory_passed: 0,
                advisory_failed: 1,
            },
        };

        let response = format_gate_response(&summary, false);
        assert_eq!(response.advisory_failed, 1);
        assert_eq!(response.required_failed, 0);
        assert!(
            !response.blocked,
            "blocked should be false when advisory fails but required passes"
        );
    }

    #[test]
    fn test_gate_run_params_with_timeout() {
        let json = serde_json::json!({
            "name": "test-spec",
            "include_evidence": true,
            "timeout": 60
        });
        let params: GateRunParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.name, "test-spec");
        assert!(params.include_evidence);
        assert_eq!(params.timeout, Some(60));
    }

    #[test]
    fn test_gate_run_params_without_timeout() {
        let json = serde_json::json!({
            "name": "test-spec"
        });
        let params: GateRunParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.name, "test-spec");
        assert!(!params.include_evidence);
        assert_eq!(params.timeout, None);
    }

    // ── gate_history tests ──────────────────────────────────────────

    #[test]
    fn test_gate_history_params_name_only() {
        let json = serde_json::json!({ "name": "auth-flow" });
        let params: GateHistoryParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.name, "auth-flow");
        assert_eq!(params.run_id, None);
        assert_eq!(params.limit, None);
    }

    #[test]
    fn test_gate_history_params_with_run_id() {
        let json = serde_json::json!({
            "name": "auth-flow",
            "run_id": "20260305T120000Z-abc123"
        });
        let params: GateHistoryParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.name, "auth-flow");
        assert_eq!(params.run_id, Some("20260305T120000Z-abc123".to_string()));
        assert_eq!(params.limit, None);
    }

    #[test]
    fn test_gate_history_params_with_limit() {
        let json = serde_json::json!({
            "name": "auth-flow",
            "limit": 5
        });
        let params: GateHistoryParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.name, "auth-flow");
        assert_eq!(params.run_id, None);
        assert_eq!(params.limit, Some(5));
    }

    #[test]
    fn test_gate_history_params_missing_name() {
        let json = serde_json::json!({ "run_id": "20260305T120000Z-abc123" });
        let err = serde_json::from_value::<GateHistoryParams>(json)
            .err()
            .expect("should fail when name is missing");
        let msg = err.to_string();
        assert!(
            msg.contains("missing field"),
            "should mention missing field: {msg}"
        );
        assert!(msg.contains("name"), "should name the missing field: {msg}");
    }

    #[test]
    fn test_gate_history_list_response_serialization() {
        let response = GateHistoryListResponse {
            spec_name: "auth-flow".to_string(),
            total_runs: 25,
            runs: vec![
                GateHistoryEntry {
                    run_id: "20260305T120000Z-abc123".to_string(),
                    timestamp: "2026-03-05T12:00:00+00:00".to_string(),
                    passed: 3,
                    failed: 1,
                    skipped: 0,
                    required_passed: 3,
                    required_failed: 1,
                    advisory_passed: 0,
                    advisory_failed: 0,
                    blocked: true,
                },
                GateHistoryEntry {
                    run_id: "20260305T110000Z-def456".to_string(),
                    timestamp: "2026-03-05T11:00:00+00:00".to_string(),
                    passed: 4,
                    failed: 0,
                    skipped: 0,
                    required_passed: 4,
                    required_failed: 0,
                    advisory_passed: 0,
                    advisory_failed: 0,
                    blocked: false,
                },
            ],
            warnings: Vec::new(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["spec_name"], "auth-flow");
        assert_eq!(json["total_runs"], 25);
        assert_eq!(json["runs"].as_array().unwrap().len(), 2);

        let first = &json["runs"][0];
        assert_eq!(first["run_id"], "20260305T120000Z-abc123");
        assert_eq!(first["passed"], 3);
        assert_eq!(first["failed"], 1);
        assert_eq!(first["required_failed"], 1);
        assert_eq!(first["blocked"], true);

        let second = &json["runs"][1];
        assert_eq!(second["blocked"], false);
        assert_eq!(second["required_failed"], 0);
    }

    #[test]
    fn test_gate_history_list_response_empty() {
        let response = GateHistoryListResponse {
            spec_name: "no-history".to_string(),
            total_runs: 0,
            runs: vec![],
            warnings: Vec::new(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["spec_name"], "no-history");
        assert_eq!(json["total_runs"], 0);
        assert!(json["runs"].as_array().unwrap().is_empty());
    }

    // ── working_dir validation test ─────────────────────────────────

    #[test]
    fn test_working_dir_nonexistent_is_not_dir() {
        let nonexistent = std::path::PathBuf::from("/tmp/assay-nonexistent-dir-12345");
        assert!(
            !nonexistent.is_dir(),
            "test precondition: path must not exist"
        );
    }

    // ── Handler test helpers ─────────────────────────────────────────

    /// Extract text content from a CallToolResult for assertion purposes.
    fn extract_text(result: &CallToolResult) -> String {
        result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    // ── Async handler tests ──────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn spec_list_valid_project_returns_specs() {
        let dir = create_project(r#"project_name = "handler-test""#);
        create_spec(
            dir.path(),
            "specs",
            "auth-flow.toml",
            r#"
name = "auth-flow"
description = "Authentication flow"

[[criteria]]
name = "unit-tests"
description = "Tests pass"
cmd = "echo ok"

[[criteria]]
name = "lint"
description = "Lint passes"
cmd = "echo lint-ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server.spec_list().await.unwrap();

        assert!(
            !result.is_error.unwrap_or(false),
            "spec_list should succeed"
        );
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(json["specs"][0]["name"], "auth-flow");
        assert_eq!(json["specs"][0]["criteria_count"], 2);
        assert_eq!(json["specs"][0]["format"], "legacy");

        insta::assert_json_snapshot!("spec_list_valid_project", json);
    }

    #[tokio::test]
    #[serial]
    async fn spec_get_valid_spec_returns_content() {
        let dir = create_project(r#"project_name = "handler-test""#);
        create_spec(
            dir.path(),
            "specs",
            "auth-flow.toml",
            r#"
name = "auth-flow"
description = "Auth spec"

[[criteria]]
name = "unit-tests"
description = "Tests pass"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .spec_get(Parameters(SpecGetParams {
                name: "auth-flow".to_string(),
                resolve: false,
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false), "spec_get should succeed");
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(json["format"], "legacy");
        assert_eq!(json["name"], "auth-flow");
        assert!(!json["criteria"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn spec_get_missing_spec_returns_error() {
        let dir = create_project(r#"project_name = "handler-test""#);
        std::fs::create_dir_all(dir.path().join(".assay").join("specs")).unwrap();

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .spec_get(Parameters(SpecGetParams {
                name: "nonexistent".to_string(),
                resolve: false,
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "spec_get for missing spec should return error"
        );
        let text = extract_text(&result);
        assert!(
            text.contains("No specs found"),
            "error should contain 'No specs found', got: {text}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn gate_run_command_spec_returns_results() {
        let dir = create_project(r#"project_name = "handler-test""#);
        create_spec(
            dir.path(),
            "specs",
            "cmd-spec.toml",
            r#"
name = "cmd-spec"
description = "Command spec"

[[criteria]]
name = "echo-check"
description = "Echo passes"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .gate_run(Parameters(GateRunParams {
                name: "cmd-spec".to_string(),
                include_evidence: false,
                timeout: Some(30),
            }))
            .await
            .unwrap();

        assert!(
            !result.is_error.unwrap_or(false),
            "gate_run should succeed, got: {}",
            extract_text(&result)
        );
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(json["spec_name"], "cmd-spec");
        assert_eq!(json["passed"], 1);
        assert_eq!(json["failed"], 0);
        assert!(!json["blocked"].as_bool().unwrap());

        // Normalize dynamic fields for snapshot stability.
        let mut snap = json.clone();
        snap["total_duration_ms"] = serde_json::json!("[duration]");
        if let Some(criteria) = snap["criteria"].as_array_mut() {
            for c in criteria {
                if c.get("duration_ms").is_some() {
                    c["duration_ms"] = serde_json::json!("[duration]");
                }
            }
        }
        insta::assert_json_snapshot!("gate_run_command_spec", snap);
    }

    #[tokio::test]
    #[serial]
    async fn gate_run_nonexistent_spec_returns_error() {
        let dir = create_project(r#"project_name = "handler-test""#);
        std::fs::create_dir_all(dir.path().join(".assay").join("specs")).unwrap();

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .gate_run(Parameters(GateRunParams {
                name: "nonexistent".to_string(),
                include_evidence: false,
                timeout: None,
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "gate_run for missing spec should return error"
        );
        let text = extract_text(&result);
        assert!(
            text.contains("No specs found"),
            "error should contain 'No specs found', got: {text}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn gate_run_nonexistent_working_dir_returns_error() {
        let dir = create_project(
            r#"
project_name = "handler-test"

[gates]
working_dir = "/tmp/assay-nonexistent-test-dir-99999"
"#,
        );
        create_spec(
            dir.path(),
            "specs",
            "cmd-spec.toml",
            r#"
name = "cmd-spec"
description = "Command spec"

[[criteria]]
name = "check"
description = "Check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .gate_run(Parameters(GateRunParams {
                name: "cmd-spec".to_string(),
                include_evidence: false,
                timeout: None,
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "gate_run with nonexistent working_dir should return error"
        );
        let text = extract_text(&result);
        assert!(
            text.contains("working directory does not exist or is not a directory"),
            "error should describe the invalid working dir, got: {text}"
        );
    }

    #[tokio::test]
    async fn gate_report_invalid_session_returns_error() {
        let server = AssayServer::new();
        let result = server
            .gate_report(Parameters(GateReportParams {
                session_id: "fake-session-id-12345".to_string(),
                criterion_name: "some-criterion".to_string(),
                passed: true,
                evidence: "observed something".to_string(),
                reasoning: "therefore it passes".to_string(),
                confidence: Some(Confidence::High),
                evaluator_role: EvaluatorRole::SelfEval,
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "gate_report for invalid session should return error"
        );
        let text = extract_text(&result);
        assert!(
            text.contains("fake-session-id-12345"),
            "error should mention the session_id, got: {text}"
        );
        assert!(
            text.contains("not found"),
            "error should say session not found, got: {text}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn gate_history_no_history_returns_empty() {
        let dir = create_project(r#"project_name = "handler-test""#);
        create_spec(
            dir.path(),
            "specs",
            "no-runs.toml",
            r#"
name = "no-runs"

[[criteria]]
name = "check"
description = "Check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .gate_history(Parameters(GateHistoryParams {
                name: "no-runs".to_string(),
                run_id: None,
                limit: None,
                outcome: None,
            }))
            .await
            .unwrap();

        assert!(
            !result.is_error.unwrap_or(false),
            "gate_history should succeed even with no history, got: {}",
            extract_text(&result)
        );
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(json["spec_name"], "no-runs");
        assert_eq!(json["total_runs"], 0);
        assert!(json["runs"].as_array().unwrap().is_empty());
    }

    // ── context_diagnose / estimate_tokens param tests ───────────────

    #[test]
    fn test_context_diagnose_params_without_session_id() {
        let json = serde_json::json!({});
        let params: ContextDiagnoseParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.session_id, None);
    }

    #[test]
    fn test_context_diagnose_params_with_session_id() {
        let json = serde_json::json!({
            "session_id": "3201041c-df85-4c91-a485-7b8c189f7636"
        });
        let params: ContextDiagnoseParams = serde_json::from_value(json).unwrap();
        assert_eq!(
            params.session_id,
            Some("3201041c-df85-4c91-a485-7b8c189f7636".to_string())
        );
    }

    #[test]
    fn test_estimate_tokens_params_without_session_id() {
        let json = serde_json::json!({});
        let params: EstimateTokensParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.session_id, None);
    }

    #[test]
    fn test_estimate_tokens_params_with_session_id() {
        let json = serde_json::json!({
            "session_id": "abc-def-123"
        });
        let params: EstimateTokensParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.session_id, Some("abc-def-123".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn context_diagnose_no_session_dir_returns_error() {
        // Use a temp dir that has no Claude Code sessions
        let dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .context_diagnose(Parameters(ContextDiagnoseParams { session_id: None }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "context_diagnose should return error when no session dir exists, got: {}",
            extract_text(&result)
        );
    }

    #[tokio::test]
    #[serial]
    async fn estimate_tokens_no_session_dir_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .estimate_tokens(Parameters(EstimateTokensParams { session_id: None }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "estimate_tokens should return error when no session dir exists, got: {}",
            extract_text(&result)
        );
    }

    // ── MCP-01: Missing required parameter tests ────────────────────

    #[test]
    fn test_gate_run_params_missing_name() {
        let json = serde_json::json!({});
        let err = serde_json::from_value::<GateRunParams>(json).err().unwrap();
        let msg = err.to_string();
        assert!(
            msg.contains("missing field"),
            "should mention missing field: {msg}"
        );
        assert!(msg.contains("name"), "should name the parameter: {msg}");
    }

    #[test]
    fn test_gate_report_params_missing_fields() {
        let json = serde_json::json!({});
        let err = serde_json::from_value::<GateReportParams>(json)
            .err()
            .unwrap();
        let msg = err.to_string();
        assert!(
            msg.contains("missing field"),
            "should mention missing field: {msg}"
        );
        assert!(
            msg.contains("session_id"),
            "should name the first required parameter: {msg}"
        );
    }

    #[test]
    fn test_spec_get_params_missing_name() {
        let json = serde_json::json!({});
        let err = serde_json::from_value::<SpecGetParams>(json).err().unwrap();
        let msg = err.to_string();
        assert!(
            msg.contains("missing field"),
            "should mention missing field: {msg}"
        );
        assert!(msg.contains("name"), "should name the parameter: {msg}");
    }

    #[test]
    fn test_gate_finalize_params_missing_session_id() {
        let json = serde_json::json!({});
        let err = serde_json::from_value::<GateFinalizeParams>(json)
            .err()
            .unwrap();
        let msg = err.to_string();
        assert!(
            msg.contains("missing field"),
            "should mention missing field: {msg}"
        );
        assert!(
            msg.contains("session_id"),
            "should name the parameter: {msg}"
        );
    }

    // ── MCP-02: Invalid parameter type tests ────────────────────────

    #[test]
    fn test_gate_run_params_invalid_timeout_type() {
        let json = serde_json::json!({"name": "test", "timeout": "abc"});
        let err = serde_json::from_value::<GateRunParams>(json)
            .err()
            .expect("should fail when timeout is a string");
        let msg = err.to_string();
        assert!(
            msg.contains("invalid type"),
            "should mention invalid type: {msg}"
        );
    }

    #[test]
    fn test_gate_report_params_invalid_passed_type() {
        let json = serde_json::json!({
            "session_id": "s",
            "criterion_name": "c",
            "passed": "yes",
            "evidence": "e",
            "reasoning": "r",
            "evaluator_role": "self"
        });
        let err = serde_json::from_value::<GateReportParams>(json)
            .err()
            .expect("should fail when passed is a string instead of bool");
        let msg = err.to_string();
        assert!(
            msg.contains("invalid type"),
            "should mention invalid type: {msg}"
        );
        assert!(
            msg.contains("bool"),
            "should mention expected type bool: {msg}"
        );
    }

    #[test]
    fn test_gate_run_params_invalid_include_evidence_type() {
        let json = serde_json::json!({"name": "test", "include_evidence": 42});
        let err = serde_json::from_value::<GateRunParams>(json)
            .err()
            .expect("should fail when include_evidence is a number instead of bool");
        let msg = err.to_string();
        assert!(
            msg.contains("invalid type"),
            "should mention invalid type: {msg}"
        );
        assert!(
            msg.contains("bool"),
            "should mention expected type bool: {msg}"
        );
    }

    // ── Truncation visibility tests ──────────────────────────────────

    #[test]
    fn test_criterion_summary_truncation_fields_all_states() {
        let response = GateRunResponse {
            spec_name: "trunc-test".to_string(),
            passed: 1,
            failed: 1,
            skipped: 1,
            required_passed: 1,
            required_failed: 1,
            advisory_passed: 0,
            advisory_failed: 0,
            blocked: true,
            total_duration_ms: 1000,
            criteria: vec![
                // Passed + truncated
                CriterionSummary {
                    name: "big-output".to_string(),
                    status: "passed".to_string(),
                    enforcement: "required".to_string(),
                    kind_label: Some("cmd".to_string()),
                    exit_code: Some(0),
                    duration_ms: Some(500),
                    reason: None,
                    stdout: None,
                    stderr: None,
                    truncated: Some(true),
                    original_bytes: Some(524_288),
                },
                // Failed + not truncated
                CriterionSummary {
                    name: "lint".to_string(),
                    status: "failed".to_string(),
                    enforcement: "required".to_string(),
                    kind_label: Some("cmd".to_string()),
                    exit_code: Some(1),
                    duration_ms: Some(500),
                    reason: Some("error".to_string()),
                    stdout: None,
                    stderr: None,
                    truncated: Some(false),
                    original_bytes: None,
                },
                // Skipped
                CriterionSummary {
                    name: "review".to_string(),
                    status: "skipped".to_string(),
                    enforcement: "advisory".to_string(),
                    kind_label: None,
                    exit_code: None,
                    duration_ms: None,
                    reason: None,
                    stdout: None,
                    stderr: None,
                    truncated: None,
                    original_bytes: None,
                },
            ],
            session_id: None,
            pending_criteria: None,
            warnings: Vec::new(),
        };

        // Struct-level assertions
        assert_eq!(response.criteria[0].truncated, Some(true));
        assert_eq!(response.criteria[0].original_bytes, Some(524_288));
        assert_eq!(response.criteria[1].truncated, Some(false));
        assert_eq!(response.criteria[1].original_bytes, None);
        assert_eq!(response.criteria[2].truncated, None);
        assert_eq!(response.criteria[2].original_bytes, None);

        // JSON serialization assertions
        let json = serde_json::to_value(&response).unwrap();

        // Passed + truncated: both fields present
        assert_eq!(json["criteria"][0]["truncated"], true);
        assert_eq!(json["criteria"][0]["original_bytes"], 524_288);

        // Failed + not truncated: both fields absent (Some(false) skipped like None)
        assert!(
            json["criteria"][1].get("truncated").is_none(),
            "non-truncated criterion should omit truncated: {}",
            json["criteria"][1]
        );
        assert!(
            json["criteria"][1].get("original_bytes").is_none(),
            "original_bytes should be omitted when None: {}",
            json["criteria"][1]
        );

        // Skipped: both fields absent
        assert!(
            json["criteria"][2].get("truncated").is_none(),
            "skipped criterion should omit truncated: {}",
            json["criteria"][2]
        );
        assert!(
            json["criteria"][2].get("original_bytes").is_none(),
            "skipped criterion should omit original_bytes: {}",
            json["criteria"][2]
        );
    }

    #[test]
    fn test_truncation_fields_independent_of_include_evidence() {
        // Build a summary with a truncated criterion
        let summary = GateRunSummary {
            spec_name: "trunc-evidence-test".to_string(),
            results: vec![
                CriterionResult {
                    criterion_name: "big-test".to_string(),
                    result: Some(GateResult {
                        passed: true,
                        kind: GateKind::Command {
                            cmd: "cargo test".to_string(),
                        },
                        stdout: "output...".to_string(),
                        stderr: String::new(),
                        exit_code: Some(0),
                        duration_ms: 200,
                        timestamp: Utc::now(),
                        truncated: true,
                        original_bytes: Some(524_288),
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Required,
                },
                // Failed + truncated — exercises the failed match arm
                CriterionResult {
                    criterion_name: "failing-test".to_string(),
                    result: Some(GateResult {
                        passed: false,
                        kind: GateKind::Command {
                            cmd: "cargo clippy".to_string(),
                        },
                        stdout: String::new(),
                        stderr: "error: lint failure".to_string(),
                        exit_code: Some(1),
                        duration_ms: 100,
                        timestamp: Utc::now(),
                        truncated: true,
                        original_bytes: Some(1_048_576),
                        evidence: None,
                        reasoning: None,
                        confidence: None,
                        evaluator_role: None,
                    }),
                    enforcement: Enforcement::Required,
                },
            ],
            passed: 1,
            failed: 1,
            skipped: 0,
            total_duration_ms: 300,
            enforcement: EnforcementSummary {
                required_passed: 1,
                required_failed: 1,
                advisory_passed: 0,
                advisory_failed: 0,
            },
        };

        // Without evidence: truncation fields still present
        let without_evidence = format_gate_response(&summary, false);
        assert_eq!(without_evidence.criteria[0].truncated, Some(true));
        assert_eq!(without_evidence.criteria[0].original_bytes, Some(524_288));
        assert!(
            without_evidence.criteria[0].stdout.is_none(),
            "stdout should be absent without evidence"
        );

        let json_no_ev = serde_json::to_value(&without_evidence).unwrap();
        assert_eq!(json_no_ev["criteria"][0]["truncated"], true);
        assert_eq!(json_no_ev["criteria"][0]["original_bytes"], 524_288);

        // Failed arm: truncation fields present without evidence
        assert_eq!(without_evidence.criteria[1].truncated, Some(true));
        assert_eq!(without_evidence.criteria[1].original_bytes, Some(1_048_576));

        // With evidence: truncation fields still present
        let with_evidence = format_gate_response(&summary, true);
        assert_eq!(with_evidence.criteria[0].truncated, Some(true));
        assert_eq!(with_evidence.criteria[0].original_bytes, Some(524_288));
        assert!(
            with_evidence.criteria[0].stdout.is_some(),
            "stdout should be present with evidence"
        );

        // Failed arm: truncation fields present with evidence
        assert_eq!(with_evidence.criteria[1].truncated, Some(true));
        assert_eq!(with_evidence.criteria[1].original_bytes, Some(1_048_576));

        let json_ev = serde_json::to_value(&with_evidence).unwrap();
        assert_eq!(json_ev["criteria"][0]["truncated"], true);
        assert_eq!(json_ev["criteria"][0]["original_bytes"], 524_288);
        assert_eq!(json_ev["criteria"][1]["truncated"], true);
        assert_eq!(json_ev["criteria"][1]["original_bytes"], 1_048_576);
    }

    // ── spec_get resolve tests ───────────────────────────────────────

    #[test]
    fn test_spec_get_params_resolve_defaults_to_false() {
        let json = serde_json::json!({"name": "my-spec"});
        let params: SpecGetParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.name, "my-spec");
        assert!(!params.resolve, "resolve should default to false");
    }

    #[test]
    fn test_spec_get_params_resolve_true() {
        let json = serde_json::json!({"name": "my-spec", "resolve": true});
        let params: SpecGetParams = serde_json::from_value(json).unwrap();
        assert!(params.resolve, "resolve should be true when set");
    }

    #[test]
    fn test_spec_get_params_resolve_false_explicit() {
        let json = serde_json::json!({"name": "my-spec", "resolve": false});
        let params: SpecGetParams = serde_json::from_value(json).unwrap();
        assert!(
            !params.resolve,
            "resolve should be false when explicitly set"
        );
    }

    #[tokio::test]
    #[serial]
    async fn spec_get_resolve_false_has_no_resolved_key() {
        let dir = create_project(r#"project_name = "handler-test""#);
        create_spec(
            dir.path(),
            "specs",
            "simple.toml",
            r#"
name = "simple"
description = "Simple spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .spec_get(Parameters(SpecGetParams {
                name: "simple".to_string(),
                resolve: false,
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert!(
            json.get("resolved").is_none(),
            "resolved key should be absent when resolve=false"
        );
    }

    #[tokio::test]
    #[serial]
    async fn spec_get_resolve_true_returns_resolved_block() {
        let dir = create_project(r#"project_name = "handler-test""#);
        create_spec(
            dir.path(),
            "specs",
            "simple.toml",
            r#"
name = "simple"
description = "Simple spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .spec_get(Parameters(SpecGetParams {
                name: "simple".to_string(),
                resolve: true,
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        let resolved = json
            .get("resolved")
            .expect("resolved key should be present when resolve=true");

        // Timeout cascade shape
        let timeout = &resolved["timeout"];
        assert_eq!(timeout["effective"], 300, "default effective timeout");
        assert!(timeout["spec"].is_null(), "spec tier should be null");
        assert!(
            timeout["config"].is_null(),
            "config tier should be null when no [gates] section"
        );
        assert_eq!(timeout["default"], 300);

        // Working dir shape
        let wd = &resolved["working_dir"];
        assert!(
            wd["path"].is_string(),
            "working_dir.path should be a string"
        );
        assert!(
            wd["exists"].is_boolean(),
            "working_dir.exists should be a bool"
        );
        assert!(
            wd["accessible"].is_boolean(),
            "working_dir.accessible should be a bool"
        );
    }

    #[tokio::test]
    #[serial]
    async fn spec_get_resolve_true_with_gates_config_shows_config_timeout() {
        let dir = create_project(
            r#"
project_name = "handler-test"

[gates]
default_timeout = 120
"#,
        );
        create_spec(
            dir.path(),
            "specs",
            "gated.toml",
            r#"
name = "gated"
description = "Spec with gates config"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .spec_get(Parameters(SpecGetParams {
                name: "gated".to_string(),
                resolve: true,
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        let timeout = &json["resolved"]["timeout"];
        assert_eq!(
            timeout["effective"], 120,
            "effective should use config timeout"
        );
        assert!(timeout["spec"].is_null(), "spec tier should be null");
        assert_eq!(
            timeout["config"], 120,
            "config tier should show the configured value"
        );
        assert_eq!(timeout["default"], 300, "default tier is always 300");
    }

    #[tokio::test]
    #[serial]
    async fn spec_get_resolve_true_working_dir_exists_and_accessible() {
        let dir = create_project(r#"project_name = "handler-test""#);
        create_spec(
            dir.path(),
            "specs",
            "simple.toml",
            r#"
name = "simple"
description = "Simple spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .spec_get(Parameters(SpecGetParams {
                name: "simple".to_string(),
                resolve: true,
            }))
            .await
            .unwrap();

        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let wd = &json["resolved"]["working_dir"];

        // CWD is a temp dir that exists and is accessible
        assert_eq!(wd["exists"], true, "CWD should exist");
        assert_eq!(wd["accessible"], true, "CWD should be accessible");
    }

    #[tokio::test]
    #[serial]
    async fn spec_get_resolve_true_directory_format_returns_resolved_block() {
        let dir = create_project(r#"project_name = "handler-test""#);
        // Create a directory-format spec
        let spec_dir = dir.path().join(".assay").join("specs").join("dir-spec");
        std::fs::create_dir_all(&spec_dir).unwrap();
        std::fs::write(
            spec_dir.join("gates.toml"),
            r#"
name = "dir-spec"
description = "Directory format spec"

[[criteria]]
name = "compiles"
description = "Code compiles"
cmd = "echo ok"
"#,
        )
        .unwrap();

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .spec_get(Parameters(SpecGetParams {
                name: "dir-spec".to_string(),
                resolve: true,
            }))
            .await
            .unwrap();

        assert!(
            !result.is_error.unwrap_or(false),
            "spec_get should succeed for directory spec"
        );
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(
            json["format"], "directory",
            "should report directory format"
        );
        assert!(
            json.get("gates").is_some(),
            "directory spec should have gates key"
        );
        let resolved = json
            .get("resolved")
            .expect("resolved key should be present when resolve=true for directory specs");
        let timeout = &resolved["timeout"];
        assert_eq!(timeout["effective"], 300, "default effective timeout");
        let wd = &resolved["working_dir"];
        assert!(
            wd["path"].is_string(),
            "working_dir.path should be a string"
        );
    }

    // ── Session tool tests ───────────────────────────────────────────

    fn create_session_params(spec_name: &str) -> SessionCreateParams {
        SessionCreateParams {
            spec_name: spec_name.to_string(),
            worktree_path: PathBuf::from("/tmp/wt/test"),
            agent_command: "claude --spec test".to_string(),
            agent_model: Some("claude-sonnet-4-20250514".to_string()),
        }
    }

    #[tokio::test]
    #[serial]
    async fn session_create_happy_path() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "auth-flow.toml",
            r#"
name = "auth-flow"
description = "Auth flow spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .session_create(Parameters(create_session_params("auth-flow")))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let text = extract_text(&result);
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert!(json["session_id"].is_string());
        assert_eq!(
            json["session_id"].as_str().unwrap().len(),
            26,
            "ULID is 26 chars"
        );
        assert_eq!(json["spec_name"], "auth-flow");
        assert_eq!(json["phase"], "created");
        assert!(json["created_at"].is_string());
        // No warnings field when empty (skip_serializing_if)
        assert!(json.get("warnings").is_none() || json["warnings"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn session_create_invalid_spec() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "real.toml",
            r#"
name = "real"
description = "Real spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .session_create(Parameters(create_session_params("nonexistent")))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "should return domain error for unknown spec"
        );
    }

    #[tokio::test]
    #[serial]
    async fn session_get_happy_path() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "my-spec.toml",
            r#"
name = "my-spec"
description = "My spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create a session first
        let create_result = server
            .session_create(Parameters(create_session_params("my-spec")))
            .await
            .unwrap();
        let create_json: serde_json::Value =
            serde_json::from_str(&extract_text(&create_result)).unwrap();
        let session_id = create_json["session_id"].as_str().unwrap().to_string();

        // Get it
        let result = server
            .session_get(Parameters(SessionGetParams {
                session_id: session_id.clone(),
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(json["id"], session_id);
        assert_eq!(json["spec_name"], "my-spec");
        assert_eq!(json["phase"], "created");
        assert!(json["agent"]["command"].is_string());
    }

    #[tokio::test]
    #[serial]
    async fn session_get_not_found() {
        let dir = create_project(r#"project_name = "session-test""#);
        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .session_get(Parameters(SessionGetParams {
                session_id: "01NONEXISTENT0000000000000".to_string(),
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "should return error for nonexistent session"
        );
    }

    #[tokio::test]
    #[serial]
    async fn session_update_happy_path() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "up-spec.toml",
            r#"
name = "up-spec"
description = "Update spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create session
        let create_result = server
            .session_create(Parameters(create_session_params("up-spec")))
            .await
            .unwrap();
        let create_json: serde_json::Value =
            serde_json::from_str(&extract_text(&create_result)).unwrap();
        let session_id = create_json["session_id"].as_str().unwrap().to_string();

        // Update to agent_running
        let result = server
            .session_update(Parameters(SessionUpdateParams {
                session_id: session_id.clone(),
                phase: SessionPhase::AgentRunning,
                trigger: "agent_started".to_string(),
                notes: None,
                gate_run_ids: vec![],
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(json["session_id"], session_id);
        assert_eq!(json["previous_phase"], "created");
        assert_eq!(json["current_phase"], "agent_running");
    }

    #[tokio::test]
    #[serial]
    async fn session_update_invalid_transition() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "inv-spec.toml",
            r#"
name = "inv-spec"
description = "Invalid transition spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create session (in "created" phase)
        let create_result = server
            .session_create(Parameters(create_session_params("inv-spec")))
            .await
            .unwrap();
        let create_json: serde_json::Value =
            serde_json::from_str(&extract_text(&create_result)).unwrap();
        let session_id = create_json["session_id"].as_str().unwrap().to_string();

        // Try to skip to completed (invalid: created -> completed)
        let result = server
            .session_update(Parameters(SessionUpdateParams {
                session_id,
                phase: SessionPhase::Completed,
                trigger: "skip".to_string(),
                notes: None,
                gate_run_ids: vec![],
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "should reject invalid transition created -> completed"
        );
    }

    #[tokio::test]
    #[serial]
    async fn session_update_with_gate_run_ids() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "gr-spec.toml",
            r#"
name = "gr-spec"
description = "Gate run IDs spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create and advance to agent_running
        let create_result = server
            .session_create(Parameters(create_session_params("gr-spec")))
            .await
            .unwrap();
        let create_json: serde_json::Value =
            serde_json::from_str(&extract_text(&create_result)).unwrap();
        let session_id = create_json["session_id"].as_str().unwrap().to_string();

        // Update with gate_run_ids including a duplicate
        let result = server
            .session_update(Parameters(SessionUpdateParams {
                session_id: session_id.clone(),
                phase: SessionPhase::AgentRunning,
                trigger: "agent_started".to_string(),
                notes: None,
                gate_run_ids: vec![
                    "run-001".to_string(),
                    "run-002".to_string(),
                    "run-001".to_string(), // duplicate
                ],
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(
            json["gate_runs_count"], 2,
            "duplicates should be deduplicated"
        );

        // Verify via session_get
        let get_result = server
            .session_get(Parameters(SessionGetParams {
                session_id: session_id.clone(),
            }))
            .await
            .unwrap();
        let get_json: serde_json::Value = serde_json::from_str(&extract_text(&get_result)).unwrap();
        let gate_runs = get_json["gate_runs"].as_array().unwrap();
        assert_eq!(gate_runs.len(), 2);
        assert_eq!(gate_runs[0], "run-001");
        assert_eq!(gate_runs[1], "run-002");
    }

    #[tokio::test]
    #[serial]
    async fn session_list_empty() {
        let dir = create_project(r#"project_name = "session-test""#);
        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .session_list(Parameters(SessionListParams {
                spec_name: None,
                phase: None,
                limit: None,
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(json["total_on_disk"], 0);
        assert_eq!(json["sessions"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    #[serial]
    async fn session_list_with_spec_name_filter() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "alpha.toml",
            r#"
name = "alpha"
description = "Alpha spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );
        create_spec(
            dir.path(),
            "specs",
            "beta.toml",
            r#"
name = "beta"
description = "Beta spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create sessions for both specs
        let mut alpha_params = create_session_params("alpha");
        alpha_params.spec_name = "alpha".to_string();
        server
            .session_create(Parameters(alpha_params))
            .await
            .unwrap();

        let mut beta_params = create_session_params("beta");
        beta_params.spec_name = "beta".to_string();
        server
            .session_create(Parameters(beta_params))
            .await
            .unwrap();

        // Filter by alpha
        let result = server
            .session_list(Parameters(SessionListParams {
                spec_name: Some("alpha".to_string()),
                phase: None,
                limit: None,
            }))
            .await
            .unwrap();

        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(json["total_on_disk"], 2, "total is before filtering");
        let sessions = json["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["spec_name"], "alpha");
    }

    #[tokio::test]
    #[serial]
    async fn session_list_with_status_filter() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "status-spec.toml",
            r#"
name = "status-spec"
description = "Status filter spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create two sessions
        let mut p1 = create_session_params("status-spec");
        p1.spec_name = "status-spec".to_string();
        let r1 = server.session_create(Parameters(p1)).await.unwrap();
        let j1: serde_json::Value = serde_json::from_str(&extract_text(&r1)).unwrap();
        let id1 = j1["session_id"].as_str().unwrap().to_string();

        let mut p2 = create_session_params("status-spec");
        p2.spec_name = "status-spec".to_string();
        server.session_create(Parameters(p2)).await.unwrap();

        // Advance first session to agent_running
        server
            .session_update(Parameters(SessionUpdateParams {
                session_id: id1,
                phase: SessionPhase::AgentRunning,
                trigger: "agent_started".to_string(),
                notes: None,
                gate_run_ids: vec![],
            }))
            .await
            .unwrap();

        // Filter by created phase — should only return the second session
        let result = server
            .session_list(Parameters(SessionListParams {
                spec_name: None,
                phase: Some(SessionPhase::Created),
                limit: None,
            }))
            .await
            .unwrap();

        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        let sessions = json["sessions"].as_array().unwrap();
        assert_eq!(
            sessions.len(),
            1,
            "only one session should be in 'created' phase"
        );
        assert_eq!(sessions[0]["phase"], "created");
    }

    #[tokio::test]
    #[serial]
    async fn session_list_respects_limit() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "limit-spec.toml",
            r#"
name = "limit-spec"
description = "Limit spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create 3 sessions
        for _ in 0..3 {
            let mut p = create_session_params("limit-spec");
            p.spec_name = "limit-spec".to_string();
            server.session_create(Parameters(p)).await.unwrap();
        }

        // List with limit=1
        let result = server
            .session_list(Parameters(SessionListParams {
                spec_name: None,
                phase: None,
                limit: Some(1),
            }))
            .await
            .unwrap();

        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(
            json["total_on_disk"], 3,
            "total should reflect all sessions on disk"
        );
        assert_eq!(
            json["sessions"].as_array().unwrap().len(),
            1,
            "should respect limit=1"
        );
    }

    // ── C2: session_update not-found test ────────────────────────────

    #[tokio::test]
    #[serial]
    async fn session_update_not_found() {
        let dir = create_project(r#"project_name = "session-test""#);
        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .session_update(Parameters(SessionUpdateParams {
                session_id: "01NONEXISTENT0000000000000".to_string(),
                phase: SessionPhase::AgentRunning,
                trigger: "agent_started".to_string(),
                notes: None,
                gate_run_ids: vec![],
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "should return error for nonexistent session"
        );
        let text = extract_text(&result);
        assert!(
            text.contains("not found"),
            "error should mention 'not found', got: {text}"
        );
    }

    // ── C3: session_list limit=0 returns at least one ────────────────

    #[tokio::test]
    #[serial]
    async fn session_list_limit_zero_returns_at_least_one() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "clamp-spec.toml",
            r#"
name = "clamp-spec"
description = "Clamp limit spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create a session
        let mut p = create_session_params("clamp-spec");
        p.spec_name = "clamp-spec".to_string();
        server.session_create(Parameters(p)).await.unwrap();

        // List with limit=0 — should clamp to 1
        let result = server
            .session_list(Parameters(SessionListParams {
                spec_name: None,
                phase: None,
                limit: Some(0),
            }))
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert!(
            !json["sessions"].as_array().unwrap().is_empty(),
            "limit=0 should clamp to 1, returning at least one session"
        );
    }

    // ── I12: session_update full lifecycle test ──────────────────────

    #[tokio::test]
    #[serial]
    async fn session_update_full_lifecycle() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "lifecycle-spec.toml",
            r#"
name = "lifecycle-spec"
description = "Full lifecycle spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create session
        let mut p = create_session_params("lifecycle-spec");
        p.spec_name = "lifecycle-spec".to_string();
        let create_result = server.session_create(Parameters(p)).await.unwrap();
        let create_json: serde_json::Value =
            serde_json::from_str(&extract_text(&create_result)).unwrap();
        let session_id = create_json["session_id"].as_str().unwrap().to_string();

        // Transition 1: created → agent_running
        let result = server
            .session_update(Parameters(SessionUpdateParams {
                session_id: session_id.clone(),
                phase: SessionPhase::AgentRunning,
                trigger: "agent_started".to_string(),
                notes: None,
                gate_run_ids: vec![],
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(json["previous_phase"], "created");
        assert_eq!(json["current_phase"], "agent_running");

        // Transition 2: agent_running → gate_evaluated
        let result = server
            .session_update(Parameters(SessionUpdateParams {
                session_id: session_id.clone(),
                phase: SessionPhase::GateEvaluated,
                trigger: "gate_run:run-001".to_string(),
                notes: Some("all criteria passed".to_string()),
                gate_run_ids: vec!["run-001".to_string()],
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(json["previous_phase"], "agent_running");
        assert_eq!(json["current_phase"], "gate_evaluated");

        // Transition 3: gate_evaluated → completed
        let result = server
            .session_update(Parameters(SessionUpdateParams {
                session_id: session_id.clone(),
                phase: SessionPhase::Completed,
                trigger: "auto_complete".to_string(),
                notes: None,
                gate_run_ids: vec![],
            }))
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        assert_eq!(json["previous_phase"], "gate_evaluated");
        assert_eq!(json["current_phase"], "completed");

        // Verify via session_get that all 3 transitions are recorded
        let get_result = server
            .session_get(Parameters(SessionGetParams {
                session_id: session_id.clone(),
            }))
            .await
            .unwrap();
        assert!(!get_result.is_error.unwrap_or(false));
        let get_json: serde_json::Value = serde_json::from_str(&extract_text(&get_result)).unwrap();
        assert_eq!(get_json["phase"], "completed");
        assert_eq!(
            get_json["transitions"].as_array().unwrap().len(),
            3,
            "should have 3 transitions"
        );
    }

    // ── I13: terminal-phase rejection test ───────────────────────────

    #[tokio::test]
    #[serial]
    async fn session_update_terminal_phase_rejected() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "terminal-spec.toml",
            r#"
name = "terminal-spec"
description = "Terminal phase spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create and advance to completed
        let mut p = create_session_params("terminal-spec");
        p.spec_name = "terminal-spec".to_string();
        let create_result = server.session_create(Parameters(p)).await.unwrap();
        let create_json: serde_json::Value =
            serde_json::from_str(&extract_text(&create_result)).unwrap();
        let session_id = create_json["session_id"].as_str().unwrap().to_string();

        // created → agent_running → gate_evaluated → completed
        for (phase, trigger) in [
            (SessionPhase::AgentRunning, "agent_started"),
            (SessionPhase::GateEvaluated, "gate_passed"),
            (SessionPhase::Completed, "auto_complete"),
        ] {
            let result = server
                .session_update(Parameters(SessionUpdateParams {
                    session_id: session_id.clone(),
                    phase,
                    trigger: trigger.to_string(),
                    notes: None,
                    gate_run_ids: vec![],
                }))
                .await
                .unwrap();
            assert!(!result.is_error.unwrap_or(false));
        }

        // Now try to transition from completed → agent_running (should fail)
        let result = server
            .session_update(Parameters(SessionUpdateParams {
                session_id: session_id.clone(),
                phase: SessionPhase::AgentRunning,
                trigger: "retry".to_string(),
                notes: None,
                gate_run_ids: vec![],
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "should reject transition from terminal phase"
        );
    }

    // ── I14: combined filter test ────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn session_list_combined_filters() {
        let dir = create_project(r#"project_name = "session-test""#);
        create_spec(
            dir.path(),
            "specs",
            "combo-a.toml",
            r#"
name = "combo-a"
description = "Combo A spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );
        create_spec(
            dir.path(),
            "specs",
            "combo-b.toml",
            r#"
name = "combo-b"
description = "Combo B spec"

[[criteria]]
name = "check"
description = "A check"
cmd = "echo ok"
"#,
        );

        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();

        // Create sessions: combo-a (created), combo-a (agent_running), combo-b (created)
        let mut pa1 = create_session_params("combo-a");
        pa1.spec_name = "combo-a".to_string();
        server.session_create(Parameters(pa1)).await.unwrap();

        let mut pa2 = create_session_params("combo-a");
        pa2.spec_name = "combo-a".to_string();
        let r2 = server.session_create(Parameters(pa2)).await.unwrap();
        let j2: serde_json::Value = serde_json::from_str(&extract_text(&r2)).unwrap();
        let id2 = j2["session_id"].as_str().unwrap().to_string();

        // Advance second combo-a session to agent_running
        server
            .session_update(Parameters(SessionUpdateParams {
                session_id: id2,
                phase: SessionPhase::AgentRunning,
                trigger: "agent_started".to_string(),
                notes: None,
                gate_run_ids: vec![],
            }))
            .await
            .unwrap();

        let mut pb = create_session_params("combo-b");
        pb.spec_name = "combo-b".to_string();
        server.session_create(Parameters(pb)).await.unwrap();

        // Filter by spec_name=combo-a AND phase=created → should return exactly 1
        let result = server
            .session_list(Parameters(SessionListParams {
                spec_name: Some("combo-a".to_string()),
                phase: Some(SessionPhase::Created),
                limit: None,
            }))
            .await
            .unwrap();

        let json: serde_json::Value = serde_json::from_str(&extract_text(&result)).unwrap();
        let sessions = json["sessions"].as_array().unwrap();
        assert_eq!(
            sessions.len(),
            1,
            "should return exactly 1 session matching both filters, got: {sessions:?}"
        );
        assert_eq!(sessions[0]["spec_name"], "combo-a");
        assert_eq!(sessions[0]["phase"], "created");
    }

    // ── run_manifest tests ───────────────────────────────────────────

    #[test]
    fn run_manifest_params_deserializes() {
        let json = r#"{"manifest_path": "test.toml", "timeout_secs": 300}"#;
        let params: RunManifestParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.manifest_path, "test.toml");
        assert_eq!(params.timeout_secs, Some(300));
    }

    #[test]
    fn run_manifest_params_deserializes_minimal() {
        let json = r#"{"manifest_path": "test.toml"}"#;
        let params: RunManifestParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.manifest_path, "test.toml");
        assert_eq!(params.timeout_secs, None);
    }

    #[test]
    fn run_manifest_params_schema_generates() {
        let schema = schemars::schema_for!(RunManifestParams);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(
            json.contains("manifest_path"),
            "schema should contain manifest_path"
        );
        assert!(
            json.contains("timeout_secs"),
            "schema should contain timeout_secs"
        );
    }

    #[tokio::test]
    #[serial]
    async fn run_manifest_tool_in_router() {
        let server = AssayServer::new();
        let tools = server.tool_router.list_all();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            tool_names.contains(&"run_manifest"),
            "run_manifest should be in tool list, got: {tool_names:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn run_manifest_missing_manifest_file() {
        let dir = create_project(r#"project_name = "run-test""#);
        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .run_manifest(Parameters(RunManifestParams {
                manifest_path: "nonexistent.toml".to_string(),
                timeout_secs: None,
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "should fail for missing manifest"
        );
    }

    // ── orchestrate_run tests ────────────────────────────────────────

    #[test]
    fn orchestrate_run_params_deserializes() {
        let json = r#"{"manifest_path": "test.toml", "timeout_secs": 300, "failure_policy": "abort", "merge_strategy": "file_overlap"}"#;
        let params: OrchestrateRunParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.manifest_path, "test.toml");
        assert_eq!(params.timeout_secs, Some(300));
        assert_eq!(params.failure_policy.as_deref(), Some("abort"));
        assert_eq!(params.merge_strategy.as_deref(), Some("file_overlap"));
    }

    #[test]
    fn orchestrate_run_params_deserializes_minimal() {
        let json = r#"{"manifest_path": "multi.toml"}"#;
        let params: OrchestrateRunParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.manifest_path, "multi.toml");
        assert_eq!(params.timeout_secs, None);
        assert_eq!(params.failure_policy, None);
        assert_eq!(params.merge_strategy, None);
    }

    #[test]
    fn orchestrate_run_params_schema_generates() {
        let schema = schemars::schema_for!(OrchestrateRunParams);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(
            json.contains("manifest_path"),
            "schema should contain manifest_path"
        );
        assert!(
            json.contains("failure_policy"),
            "schema should contain failure_policy"
        );
        assert!(
            json.contains("merge_strategy"),
            "schema should contain merge_strategy"
        );
        assert!(
            json.contains("conflict_resolution"),
            "schema should contain conflict_resolution"
        );
    }

    #[test]
    fn orchestrate_run_params_conflict_resolution_auto_deserializes() {
        let json = r#"{"manifest_path": "multi.toml", "conflict_resolution": "auto"}"#;
        let params: OrchestrateRunParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.manifest_path, "multi.toml");
        assert_eq!(
            params.conflict_resolution.as_deref(),
            Some("auto"),
            "conflict_resolution should be 'auto'"
        );
    }

    #[test]
    fn orchestrate_run_params_conflict_resolution_skip_deserializes() {
        let json = r#"{"manifest_path": "multi.toml", "conflict_resolution": "skip"}"#;
        let params: OrchestrateRunParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.conflict_resolution.as_deref(), Some("skip"));
    }

    #[test]
    fn orchestrate_run_params_conflict_resolution_defaults_to_none() {
        let json = r#"{"manifest_path": "multi.toml"}"#;
        let params: OrchestrateRunParams = serde_json::from_str(json).unwrap();
        assert!(
            params.conflict_resolution.is_none(),
            "conflict_resolution should be None when omitted (defaults to skip behavior)"
        );
    }

    #[tokio::test]
    #[serial]
    async fn orchestrate_run_tool_in_router() {
        let server = AssayServer::new();
        let tools = server.tool_router.list_all();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            tool_names.contains(&"orchestrate_run"),
            "orchestrate_run should be in tool list, got: {tool_names:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn orchestrate_run_missing_manifest() {
        let dir = create_project(r#"project_name = "orch-test""#);
        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .orchestrate_run(Parameters(OrchestrateRunParams {
                manifest_path: "nonexistent.toml".to_string(),
                timeout_secs: None,
                failure_policy: None,
                merge_strategy: None,
                conflict_resolution: None,
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "should fail for missing manifest"
        );
    }

    #[tokio::test]
    #[serial]
    async fn orchestrate_run_mesh_skips_session_count_guard() {
        // A single-session manifest with mode = "mesh" must NOT be rejected by
        // the multi-session guard (which only applies to DAG mode).
        let dir = create_project(r#"project_name = "mesh-test""#);
        std::env::set_current_dir(dir.path()).unwrap();

        // Write a single-session mesh manifest.
        let manifest_content = r#"mode = "mesh"
[[sessions]]
spec = "auth"
"#;
        let manifest_path = dir.path().join("mesh.toml");
        std::fs::write(&manifest_path, manifest_content).unwrap();

        let server = AssayServer::new();
        let result = server
            .orchestrate_run(Parameters(OrchestrateRunParams {
                manifest_path: manifest_path.to_string_lossy().to_string(),
                timeout_secs: None,
                failure_policy: None,
                merge_strategy: None,
                conflict_resolution: None,
            }))
            .await
            .unwrap();

        // The guard should NOT reject this — it will fail for other reasons
        // (missing spec file), but NOT with the "must contain multiple sessions"
        // error that the DAG guard produces.
        let text = result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        assert!(
            !text.contains("must contain multiple sessions"),
            "mesh mode should not trigger the DAG multi-session guard; got: {text}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn orchestrate_run_gossip_skips_session_count_guard() {
        let dir = create_project(r#"project_name = "gossip-test""#);
        std::env::set_current_dir(dir.path()).unwrap();

        let manifest_content = r#"mode = "gossip"
[[sessions]]
spec = "auth"
"#;
        let manifest_path = dir.path().join("gossip.toml");
        std::fs::write(&manifest_path, manifest_content).unwrap();

        let server = AssayServer::new();
        let result = server
            .orchestrate_run(Parameters(OrchestrateRunParams {
                manifest_path: manifest_path.to_string_lossy().to_string(),
                timeout_secs: None,
                failure_policy: None,
                merge_strategy: None,
                conflict_resolution: None,
            }))
            .await
            .unwrap();

        let text = result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        assert!(
            !text.contains("must contain multiple sessions"),
            "gossip mode should not trigger the DAG multi-session guard; got: {text}"
        );
    }

    // ── orchestrate_status tests ─────────────────────────────────────

    #[test]
    fn orchestrate_status_params_deserializes() {
        let json = r#"{"run_id": "01JTEST123"}"#;
        let params: OrchestrateStatusParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.run_id, "01JTEST123");
    }

    #[test]
    fn orchestrate_status_params_schema_generates() {
        let schema = schemars::schema_for!(OrchestrateStatusParams);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("run_id"), "schema should contain run_id");
    }

    #[tokio::test]
    #[serial]
    async fn orchestrate_status_tool_in_router() {
        let server = AssayServer::new();
        let tools = server.tool_router.list_all();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            tool_names.contains(&"orchestrate_status"),
            "orchestrate_status should be in tool list, got: {tool_names:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn orchestrate_status_missing_run_id() {
        let dir = create_project(r#"project_name = "status-test""#);
        std::env::set_current_dir(dir.path()).unwrap();
        // Create .assay dir so CWD resolves
        std::fs::create_dir_all(dir.path().join(".assay")).unwrap();

        let server = AssayServer::new();
        let result = server
            .orchestrate_status(Parameters(OrchestrateStatusParams {
                run_id: "01JNONEXISTENT".to_string(),
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "should fail for missing run_id"
        );
        let text = extract_text(&result);
        assert!(
            text.contains("No orchestrator state found"),
            "should mention missing state, got: {text}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn orchestrate_status_reads_valid_state() {
        let dir = create_project(r#"project_name = "status-read""#);
        std::env::set_current_dir(dir.path()).unwrap();

        // Write a valid state.json
        let run_id = "01JTESTREAD";
        let state_dir = dir.path().join(".assay").join("orchestrator").join(run_id);
        std::fs::create_dir_all(&state_dir).unwrap();

        let status = assay_types::OrchestratorStatus {
            run_id: run_id.to_string(),
            phase: assay_types::OrchestratorPhase::Completed,
            failure_policy: assay_types::FailurePolicy::SkipDependents,
            sessions: vec![],
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            mesh_status: None,
            gossip_status: None,
        };
        let json = serde_json::to_string_pretty(&status).unwrap();
        std::fs::write(state_dir.join("state.json"), &json).unwrap();

        let server = AssayServer::new();
        let result = server
            .orchestrate_status(Parameters(OrchestrateStatusParams {
                run_id: run_id.to_string(),
            }))
            .await
            .unwrap();

        assert!(
            !result.is_error.unwrap_or(false),
            "should succeed for valid state"
        );
        let text = extract_text(&result);
        // Response is now wrapped: { "status": {...}, "merge_report": null }
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value["status"]["run_id"], "01JTESTREAD",
            "status.run_id should match, got: {text}"
        );
        assert!(
            value.get("merge_report").is_some(),
            "merge_report key should be present (null is fine), got: {text}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn orchestrate_status_reads_merge_report_when_present() {
        let dir = create_project(r#"project_name = "status-merge-report""#);
        std::env::set_current_dir(dir.path()).unwrap();

        let run_id = "01JTESTREPORT";
        let state_dir = dir.path().join(".assay").join("orchestrator").join(run_id);
        std::fs::create_dir_all(&state_dir).unwrap();

        // Write state.json
        let status = assay_types::OrchestratorStatus {
            run_id: run_id.to_string(),
            phase: assay_types::OrchestratorPhase::Completed,
            failure_policy: assay_types::FailurePolicy::SkipDependents,
            sessions: vec![],
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            mesh_status: None,
            gossip_status: None,
        };
        let state_json = serde_json::to_string_pretty(&status).unwrap();
        std::fs::write(state_dir.join("state.json"), &state_json).unwrap();

        // Write a minimal merge_report.json
        let merge_report = assay_types::MergeReport {
            sessions_merged: 0,
            conflict_skipped: 0,
            aborted: 0,
            sessions_skipped: 0,
            duration_secs: 0.0,
            plan: assay_types::MergePlan {
                strategy: assay_types::MergeStrategy::CompletionTime,
                entries: vec![],
            },
            results: vec![],
            resolutions: vec![],
        };
        let report_json = serde_json::to_string_pretty(&merge_report).unwrap();
        std::fs::write(state_dir.join("merge_report.json"), &report_json).unwrap();

        let server = AssayServer::new();
        let result = server
            .orchestrate_status(Parameters(OrchestrateStatusParams {
                run_id: run_id.to_string(),
            }))
            .await
            .unwrap();

        assert!(
            !result.is_error.unwrap_or(false),
            "should succeed when merge_report.json is present"
        );
        let text = extract_text(&result);
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value["status"]["run_id"], "01JTESTREPORT",
            "status.run_id should match, got: {text}"
        );
        assert!(
            value["merge_report"].is_object(),
            "merge_report should be an object (not null), got: {text}"
        );
        assert_eq!(
            value["merge_report"]["sessions_merged"], 0,
            "sessions_merged should be 0, got: {text}"
        );
    }

    // ── milestone_list tests ─────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn milestone_list_tool_in_router() {
        let server = AssayServer::new();
        let tools = server.tool_router.list_all();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            tool_names.contains(&"milestone_list"),
            "milestone_list should be in tool list, got: {tool_names:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn milestone_get_tool_in_router() {
        let server = AssayServer::new();
        let tools = server.tool_router.list_all();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            tool_names.contains(&"milestone_get"),
            "milestone_get should be in tool list, got: {tool_names:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn milestone_list_returns_empty_json_array_for_no_milestones() {
        let dir = create_project(r#"project_name = "milestone-test""#);
        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .milestone_list(Parameters(MilestoneListParams {}))
            .await
            .unwrap();

        assert!(
            !result.is_error.unwrap_or(false),
            "milestone_list should succeed for a project with no milestones"
        );
        let text = extract_text(&result);
        let value: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("response should be valid JSON, got: {text:?}, err: {e}"));
        assert!(
            value.is_array(),
            "response should be a JSON array, got: {text}"
        );
        assert_eq!(
            value.as_array().unwrap().len(),
            0,
            "response should be an empty array for no milestones, got: {text}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn milestone_get_returns_error_for_missing_slug() {
        let dir = create_project(r#"project_name = "milestone-test""#);
        std::env::set_current_dir(dir.path()).unwrap();

        let server = AssayServer::new();
        let result = server
            .milestone_get(Parameters(MilestoneGetParams {
                slug: "nonexistent".to_string(),
            }))
            .await
            .unwrap();

        assert!(
            result.is_error.unwrap_or(false),
            "milestone_get should return isError: true for a missing slug"
        );
    }

    #[tokio::test]
    #[serial]
    async fn cycle_status_tool_in_router() {
        let server = AssayServer::new();
        let tools = server.tool_router.list_all();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            tool_names.contains(&"cycle_status"),
            "cycle_status should be in tool list, got: {tool_names:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn cycle_advance_tool_in_router() {
        let server = AssayServer::new();
        let tools = server.tool_router.list_all();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            tool_names.contains(&"cycle_advance"),
            "cycle_advance should be in tool list, got: {tool_names:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn chunk_status_tool_in_router() {
        let server = AssayServer::new();
        let tools = server.tool_router.list_all();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            tool_names.contains(&"chunk_status"),
            "chunk_status should be in tool list, got: {tool_names:?}"
        );
    }
}
