//! Orchestrator state types for multi-session parallel execution.
//!
//! These types define the data contract for orchestrator state persistence,
//! consumed by the executor (S02), merge runner (S03), MCP tools (S06),
//! and CLI status commands. All types are serializable and schema-generating
//! for structured observability.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema_registry;

/// Run state of a single session in the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionRunState {
    /// Session is queued and waiting for dependencies/concurrency slot.
    Pending,
    /// Session is currently executing.
    Running,
    /// Session completed successfully.
    Completed,
    /// Session failed during execution.
    Failed,
    /// Session was skipped (e.g., upstream dependency failed).
    Skipped,
}

/// Failure policy for orchestrated runs.
///
/// Defaults to [`FailurePolicy::SkipDependents`], which allows independent
/// sessions to continue while skipping only those that depend on the failed session.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailurePolicy {
    /// Skip dependents of failed sessions, continue independent sessions.
    #[default]
    SkipDependents,
    /// Abort the entire run on first failure — no new sessions are dispatched.
    Abort,
}

/// High-level phase of an orchestrated run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrchestratorPhase {
    /// Sessions are still being dispatched/executed.
    Running,
    /// All sessions completed successfully.
    Completed,
    /// At least one session failed; others may have completed or been skipped.
    PartialFailure,
    /// Run was aborted (due to [`FailurePolicy::Abort`] or external signal).
    Aborted,
}

/// Per-session status snapshot for observability.
///
/// Persisted as part of [`OrchestratorStatus`] in
/// `.assay/orchestrator/<run_id>/state.json` and readable by the
/// `orchestrate_status` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionStatus {
    /// Effective session name (from manifest or spec name fallback).
    pub name: String,
    /// Spec path or name this session evaluates.
    pub spec: String,
    /// Current run state.
    pub state: SessionRunState,
    /// When the session started executing (None if still pending/skipped).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// When the session finished (None if still pending/running).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Wall-clock duration in seconds (None if not yet completed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    /// Error message if the session failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Reason the session was skipped (e.g., "upstream 'auth' failed").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

/// Orchestrator-level status snapshot, persisted for status queries.
///
/// Written to `.assay/orchestrator/<run_id>/state.json` after each session
/// completion. Designed to be read by the `orchestrate_status` MCP tool
/// and CLI status commands.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrchestratorStatus {
    /// Unique identifier for this orchestrated run.
    pub run_id: String,
    /// Current phase of the run.
    pub phase: OrchestratorPhase,
    /// Failure policy in effect for this run.
    pub failure_policy: FailurePolicy,
    /// Per-session status snapshots.
    pub sessions: Vec<SessionStatus>,
    /// When the orchestrated run started.
    pub started_at: DateTime<Utc>,
    /// When the run completed (None if still running).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

// ── Merge ordering & runner types ─────────────────────────────────────

/// Strategy for ordering session branches during the merge phase.
///
/// Determines the sequence in which completed session branches are merged
/// into the base branch. Order matters: merging A then B can succeed while
/// B then A conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Sort by completion timestamp, with topological index as tiebreak.
    CompletionTime,
    /// Greedy algorithm: iteratively pick the session whose changed files
    /// have the least overlap with the already-merged set.
    FileOverlap,
}

/// A single entry in the merge plan, recording why a session was placed
/// at a particular position in the merge sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergePlanEntry {
    /// Effective session name.
    pub session_name: String,
    /// 0-based position in the merge sequence.
    pub position: usize,
    /// Human-readable reason for this placement (e.g., "earliest completion" or "0 overlapping files").
    pub reason: String,
}

/// The planned merge order for a set of sessions, including the strategy
/// used and per-session rationale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergePlan {
    /// Strategy used to determine the ordering.
    pub strategy: MergeStrategy,
    /// Per-session ordering entries.
    pub entries: Vec<MergePlanEntry>,
}

/// Status of a single session after the merge phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MergeSessionStatus {
    /// Session branch was successfully merged.
    Merged,
    /// Session was skipped (e.g., not completed or filtered out before merge).
    Skipped,
    /// Session was skipped because its merge had a conflict and the handler chose to skip.
    ConflictSkipped,
    /// Merge sequence was aborted (conflict handler chose abort, or abort policy triggered).
    Aborted,
    /// Session merge failed due to an infrastructure error.
    Failed,
}

/// Per-session result from the merge phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeSessionResult {
    /// Effective session name.
    pub session_name: String,
    /// Merge outcome status.
    pub status: MergeSessionStatus,
    /// SHA of the merge commit (present only when status is `Merged`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_sha: Option<String>,
    /// Error or skip reason (present for `Failed`, `ConflictSkipped`, `Aborted`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Original or resolved content of a single file captured during conflict resolution.
///
/// Used in [`ConflictResolution`] to store both the original conflict-marker content
/// and the AI-resolved content for each conflicted file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConflictFileContent {
    /// Path relative to the repository root.
    pub path: String,
    /// Full file content (with conflict markers for originals; resolved for outputs).
    pub content: String,
}

/// Audit record for a single AI-resolved merge conflict.
///
/// Captured by the merge runner when AI conflict resolution succeeds and stored
/// in [`MergeReport::resolutions`]. Enables post-run inspection of what the AI
/// changed and whether the optional validation command accepted the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConflictResolution {
    /// The session whose branch produced the conflict.
    pub session_name: String,
    /// Paths of files that had conflict markers, relative to the repo root.
    pub conflicting_files: Vec<String>,
    /// Original file contents (with conflict markers) before AI resolution.
    pub original_contents: Vec<ConflictFileContent>,
    /// Resolved file contents after AI resolution (conflict markers removed).
    pub resolved_contents: Vec<ConflictFileContent>,
    /// Raw stdout from the resolver subprocess (AI-generated content).
    pub resolver_stdout: String,
    /// Whether the optional validation command accepted the resolution.
    ///
    /// `None` when no `validation_command` was configured.
    /// `Some(true)` when validation passed; `Some(false)` when it rejected the result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_passed: Option<bool>,
}

/// Summary report of the entire merge phase.
///
/// Provides aggregate counts and per-session details for post-run inspection.
/// Serializable for persistence alongside [`OrchestratorStatus`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeReport {
    /// Number of sessions successfully merged.
    pub sessions_merged: usize,
    /// Number of sessions skipped (pre-merge filtering).
    pub sessions_skipped: usize,
    /// Number of sessions skipped due to conflicts.
    pub conflict_skipped: usize,
    /// Number of sessions aborted.
    pub aborted: usize,
    /// The merge plan that determined ordering.
    pub plan: MergePlan,
    /// Per-session merge results in merge-order sequence.
    pub results: Vec<MergeSessionResult>,
    /// Wall-clock duration of the merge phase in seconds.
    pub duration_secs: f64,
    /// Audit records for AI-resolved conflicts, one per resolved session.
    ///
    /// Empty when no AI resolution occurred. Uses `default` + `skip_serializing_if`
    /// so pre-existing persisted reports without this field deserialize correctly.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolutions: Vec<ConflictResolution>,
}

/// Action to take when a merge conflict is detected.
///
/// Passed to the conflict handler closure. Not `deny_unknown_fields` since
/// it is operational (not a persistence contract), but has serde for logging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictAction {
    /// Conflict was resolved; the provided string is the resolution commit SHA.
    Resolved(String),
    /// Skip this session and continue with the next.
    Skip,
    /// Abort the entire merge sequence.
    Abort,
}

/// Configuration for AI-driven conflict resolution during merge.
///
/// When `enabled` is `true`, the merge runner will attempt to resolve
/// conflicts using an AI model instead of aborting or skipping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConflictResolutionConfig {
    /// Whether AI conflict resolution is active.
    pub enabled: bool,
    /// Model identifier for the resolver (e.g. `"claude-sonnet-4-20250514"`).
    #[serde(default = "default_conflict_model")]
    pub model: String,
    /// Maximum seconds to wait for the resolver before aborting.
    #[serde(default = "default_conflict_timeout")]
    pub timeout_secs: u64,
    /// Optional shell command to validate a resolution before accepting it.
    ///
    /// Run after the AI writes resolved files and stages them, but before the
    /// merge commit. A non-zero exit code causes the resolution to be rejected
    /// and the session to be skipped with `ConflictSkipped`. When `None`, no
    /// validation is performed and the resolution is accepted unconditionally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_command: Option<String>,
}

fn default_conflict_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

fn default_conflict_timeout() -> u64 {
    120
}

impl Default for ConflictResolutionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: default_conflict_model(),
            timeout_secs: default_conflict_timeout(),
            validation_command: None,
        }
    }
}

// ── Coordination mode types ───────────────────────────────────────────

/// Coordination mode for multi-session orchestrated runs.
///
/// Selects the execution topology: the default DAG mode uses explicit
/// `depends_on` edges; Mesh uses heartbeat-based peer coordination;
/// Gossip uses periodic coordinator broadcast.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrchestratorMode {
    /// Directed-acyclic-graph execution using explicit `depends_on` edges.
    /// This is the default coordination mode.
    #[default]
    Dag,
    /// Heartbeat-based peer mesh coordination.
    Mesh,
    /// Periodic gossip-based coordinator broadcast.
    Gossip,
}

/// Configuration for Mesh coordination mode.
///
/// Controls the heartbeat and timeout thresholds used when `mode = "mesh"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeshConfig {
    /// Interval in seconds between heartbeat pings. Defaults to 5.
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
    /// Seconds without a heartbeat before a peer is considered suspect. Defaults to 10.
    #[serde(default = "default_suspect_timeout")]
    pub suspect_timeout_secs: u64,
    /// Seconds without a heartbeat before a peer is considered dead. Defaults to 30.
    #[serde(default = "default_dead_timeout")]
    pub dead_timeout_secs: u64,
}

fn default_heartbeat_interval() -> u64 {
    5
}
fn default_suspect_timeout() -> u64 {
    10
}
fn default_dead_timeout() -> u64 {
    30
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_secs: default_heartbeat_interval(),
            suspect_timeout_secs: default_suspect_timeout(),
            dead_timeout_secs: default_dead_timeout(),
        }
    }
}

/// Configuration for Gossip coordination mode.
///
/// Controls the coordinator broadcast interval used when `mode = "gossip"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GossipConfig {
    /// Interval in seconds between coordinator gossip rounds. Defaults to 5.
    #[serde(default = "default_coordinator_interval")]
    pub coordinator_interval_secs: u64,
}

fn default_coordinator_interval() -> u64 {
    5
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            coordinator_interval_secs: default_coordinator_interval(),
        }
    }
}

// ── Schema registry submissions ──────────────────────────────────────

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "session-run-state",
        generate: || schemars::schema_for!(SessionRunState),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "failure-policy",
        generate: || schemars::schema_for!(FailurePolicy),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "orchestrator-phase",
        generate: || schemars::schema_for!(OrchestratorPhase),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "session-status",
        generate: || schemars::schema_for!(SessionStatus),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "orchestrator-status",
        generate: || schemars::schema_for!(OrchestratorStatus),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "merge-strategy",
        generate: || schemars::schema_for!(MergeStrategy),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "merge-plan-entry",
        generate: || schemars::schema_for!(MergePlanEntry),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "merge-plan",
        generate: || schemars::schema_for!(MergePlan),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "merge-session-status",
        generate: || schemars::schema_for!(MergeSessionStatus),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "merge-session-result",
        generate: || schemars::schema_for!(MergeSessionResult),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "merge-report",
        generate: || schemars::schema_for!(MergeReport),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "conflict-action",
        generate: || schemars::schema_for!(ConflictAction),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "conflict-resolution-config",
        generate: || schemars::schema_for!(ConflictResolutionConfig),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "conflict-file-content",
        generate: || schemars::schema_for!(ConflictFileContent),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "conflict-resolution",
        generate: || schemars::schema_for!(ConflictResolution),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "orchestrator-mode",
        generate: || schemars::schema_for!(OrchestratorMode),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "mesh-config",
        generate: || schemars::schema_for!(MeshConfig),
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "gossip-config",
        generate: || schemars::schema_for!(GossipConfig),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_run_state_serde_roundtrip() {
        let states = vec![
            SessionRunState::Pending,
            SessionRunState::Running,
            SessionRunState::Completed,
            SessionRunState::Failed,
            SessionRunState::Skipped,
        ];
        for state in &states {
            let json = serde_json::to_string(state).unwrap();
            let back: SessionRunState = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, state);
        }
        // Verify snake_case serialization
        assert_eq!(
            serde_json::to_string(&SessionRunState::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&SessionRunState::Skipped).unwrap(),
            "\"skipped\""
        );
    }

    #[test]
    fn failure_policy_serde_roundtrip() {
        let policies = vec![FailurePolicy::SkipDependents, FailurePolicy::Abort];
        for policy in &policies {
            let json = serde_json::to_string(policy).unwrap();
            let back: FailurePolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, policy);
        }
        assert_eq!(
            serde_json::to_string(&FailurePolicy::SkipDependents).unwrap(),
            "\"skip_dependents\""
        );
        assert_eq!(
            serde_json::to_string(&FailurePolicy::Abort).unwrap(),
            "\"abort\""
        );
    }

    #[test]
    fn failure_policy_default_is_skip_dependents() {
        assert_eq!(FailurePolicy::default(), FailurePolicy::SkipDependents);
    }

    #[test]
    fn orchestrator_phase_serde_roundtrip() {
        let phases = vec![
            OrchestratorPhase::Running,
            OrchestratorPhase::Completed,
            OrchestratorPhase::PartialFailure,
            OrchestratorPhase::Aborted,
        ];
        for phase in &phases {
            let json = serde_json::to_string(phase).unwrap();
            let back: OrchestratorPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, phase);
        }
        assert_eq!(
            serde_json::to_string(&OrchestratorPhase::PartialFailure).unwrap(),
            "\"partial_failure\""
        );
    }

    #[test]
    fn session_status_serde_roundtrip() {
        let status = SessionStatus {
            name: "auth-flow".to_string(),
            spec: "specs/auth.toml".to_string(),
            state: SessionRunState::Failed,
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            duration_secs: Some(42.5),
            error: Some("gate evaluation failed".to_string()),
            skip_reason: None,
        };
        let json = serde_json::to_string_pretty(&status).unwrap();
        let back: SessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "auth-flow");
        assert_eq!(back.state, SessionRunState::Failed);
        assert!(back.error.is_some());
        assert!(back.skip_reason.is_none());
    }

    #[test]
    fn session_status_skipped_roundtrip() {
        let status = SessionStatus {
            name: "payment".to_string(),
            spec: "specs/payment.toml".to_string(),
            state: SessionRunState::Skipped,
            started_at: None,
            completed_at: None,
            duration_secs: None,
            error: None,
            skip_reason: Some("upstream 'auth-flow' failed".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: SessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.state, SessionRunState::Skipped);
        assert_eq!(
            back.skip_reason.as_deref(),
            Some("upstream 'auth-flow' failed")
        );
    }

    #[test]
    fn orchestrator_status_full_roundtrip() {
        let now = Utc::now();
        let status = OrchestratorStatus {
            run_id: "01JTEST123".to_string(),
            phase: OrchestratorPhase::PartialFailure,
            failure_policy: FailurePolicy::SkipDependents,
            sessions: vec![
                SessionStatus {
                    name: "auth".to_string(),
                    spec: "specs/auth.toml".to_string(),
                    state: SessionRunState::Completed,
                    started_at: Some(now),
                    completed_at: Some(now),
                    duration_secs: Some(30.0),
                    error: None,
                    skip_reason: None,
                },
                SessionStatus {
                    name: "payment".to_string(),
                    spec: "specs/payment.toml".to_string(),
                    state: SessionRunState::Failed,
                    started_at: Some(now),
                    completed_at: Some(now),
                    duration_secs: Some(15.5),
                    error: Some("agent timeout".to_string()),
                    skip_reason: None,
                },
                SessionStatus {
                    name: "checkout".to_string(),
                    spec: "specs/checkout.toml".to_string(),
                    state: SessionRunState::Skipped,
                    started_at: None,
                    completed_at: None,
                    duration_secs: None,
                    error: None,
                    skip_reason: Some("upstream 'payment' failed".to_string()),
                },
            ],
            started_at: now,
            completed_at: Some(now),
        };
        let json = serde_json::to_string_pretty(&status).unwrap();
        let back: OrchestratorStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.run_id, "01JTEST123");
        assert_eq!(back.phase, OrchestratorPhase::PartialFailure);
        assert_eq!(back.failure_policy, FailurePolicy::SkipDependents);
        assert_eq!(back.sessions.len(), 3);
    }

    #[test]
    fn session_status_deny_unknown_fields() {
        let json = r#"{"name":"x","spec":"s","state":"pending","unknown_field":true}"#;
        let result = serde_json::from_str::<SessionStatus>(json);
        assert!(result.is_err(), "should reject unknown fields");
    }

    #[test]
    fn merge_strategy_serde_roundtrip() {
        let strategies = vec![MergeStrategy::CompletionTime, MergeStrategy::FileOverlap];
        for s in &strategies {
            let json = serde_json::to_string(s).unwrap();
            let back: MergeStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, s);
        }
        assert_eq!(
            serde_json::to_string(&MergeStrategy::CompletionTime).unwrap(),
            "\"completion_time\""
        );
        assert_eq!(
            serde_json::to_string(&MergeStrategy::FileOverlap).unwrap(),
            "\"file_overlap\""
        );
    }

    #[test]
    fn merge_plan_entry_serde_roundtrip() {
        let entry = MergePlanEntry {
            session_name: "auth".to_string(),
            position: 0,
            reason: "earliest completion".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: MergePlanEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_name, "auth");
        assert_eq!(back.position, 0);
    }

    #[test]
    fn merge_plan_entry_deny_unknown_fields() {
        let json = r#"{"session_name":"x","position":0,"reason":"y","extra":1}"#;
        assert!(serde_json::from_str::<MergePlanEntry>(json).is_err());
    }

    #[test]
    fn merge_plan_serde_roundtrip() {
        let plan = MergePlan {
            strategy: MergeStrategy::FileOverlap,
            entries: vec![MergePlanEntry {
                session_name: "auth".to_string(),
                position: 0,
                reason: "0 overlapping files".to_string(),
            }],
        };
        let json = serde_json::to_string(&plan).unwrap();
        let back: MergePlan = serde_json::from_str(&json).unwrap();
        assert_eq!(back.strategy, MergeStrategy::FileOverlap);
        assert_eq!(back.entries.len(), 1);
    }

    #[test]
    fn merge_session_status_serde_roundtrip() {
        let statuses = vec![
            MergeSessionStatus::Merged,
            MergeSessionStatus::Skipped,
            MergeSessionStatus::ConflictSkipped,
            MergeSessionStatus::Aborted,
            MergeSessionStatus::Failed,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: MergeSessionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, s);
        }
        assert_eq!(
            serde_json::to_string(&MergeSessionStatus::ConflictSkipped).unwrap(),
            "\"conflict_skipped\""
        );
    }

    #[test]
    fn merge_session_result_serde_roundtrip() {
        let result = MergeSessionResult {
            session_name: "auth".to_string(),
            status: MergeSessionStatus::Merged,
            merge_sha: Some("abc123".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: MergeSessionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_name, "auth");
        assert_eq!(back.status, MergeSessionStatus::Merged);
        assert!(back.merge_sha.is_some());
    }

    #[test]
    fn merge_report_serde_roundtrip() {
        let report = MergeReport {
            sessions_merged: 2,
            sessions_skipped: 0,
            conflict_skipped: 1,
            aborted: 0,
            plan: MergePlan {
                strategy: MergeStrategy::CompletionTime,
                entries: vec![],
            },
            results: vec![],
            duration_secs: 12.5,
            resolutions: vec![],
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        let back: MergeReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sessions_merged, 2);
        assert_eq!(back.conflict_skipped, 1);
    }

    #[test]
    fn merge_report_deny_unknown_fields() {
        let json = r#"{"sessions_merged":0,"sessions_skipped":0,"conflict_skipped":0,"aborted":0,"plan":{"strategy":"completion_time","entries":[]},"results":[],"duration_secs":0.0,"extra":1}"#;
        assert!(serde_json::from_str::<MergeReport>(json).is_err());
    }

    #[test]
    fn conflict_action_serde_roundtrip() {
        let actions = vec![
            ConflictAction::Resolved("abc123".to_string()),
            ConflictAction::Skip,
            ConflictAction::Abort,
        ];
        for a in &actions {
            let json = serde_json::to_string(a).unwrap();
            let back: ConflictAction = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, a);
        }
    }

    #[test]
    fn orchestrator_status_deny_unknown_fields() {
        let json = r#"{"run_id":"x","phase":"running","failure_policy":"abort","sessions":[],"started_at":"2026-01-01T00:00:00Z","extra":1}"#;
        let result = serde_json::from_str::<OrchestratorStatus>(json);
        assert!(result.is_err(), "should reject unknown fields");
    }

    #[test]
    fn conflict_resolution_config_default() {
        let config = ConflictResolutionConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.timeout_secs, 120);
    }

    #[test]
    fn conflict_resolution_config_serde_roundtrip() {
        let config = ConflictResolutionConfig {
            enabled: true,
            model: "custom-model".to_string(),
            timeout_secs: 60,
            validation_command: None,
        };
        let json = serde_json::to_string_pretty(&config).unwrap();
        let back: ConflictResolutionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, config);
    }

    #[test]
    fn conflict_resolution_config_defaults_applied() {
        // Only enabled is required; model and timeout_secs have defaults
        let json = r#"{"enabled":true}"#;
        let config: ConflictResolutionConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.timeout_secs, 120);
    }

    #[test]
    fn conflict_resolution_config_deny_unknown_fields() {
        let json = r#"{"enabled":false,"model":"x","timeout_secs":10,"extra":1}"#;
        assert!(
            serde_json::from_str::<ConflictResolutionConfig>(json).is_err(),
            "should reject unknown fields"
        );
    }

    // ── OrchestratorMode tests ────────────────────────────────────────

    #[test]
    fn orchestrator_mode_default_is_dag() {
        assert_eq!(OrchestratorMode::default(), OrchestratorMode::Dag);
    }

    #[test]
    fn orchestrator_mode_serde_roundtrip() {
        let modes = vec![
            OrchestratorMode::Dag,
            OrchestratorMode::Mesh,
            OrchestratorMode::Gossip,
        ];
        for mode in &modes {
            let json = serde_json::to_string(mode).unwrap();
            let back: OrchestratorMode = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, mode);
        }
        // Verify snake_case serialization
        assert_eq!(
            serde_json::to_string(&OrchestratorMode::Dag).unwrap(),
            "\"dag\""
        );
        assert_eq!(
            serde_json::to_string(&OrchestratorMode::Mesh).unwrap(),
            "\"mesh\""
        );
        assert_eq!(
            serde_json::to_string(&OrchestratorMode::Gossip).unwrap(),
            "\"gossip\""
        );
    }

    // ── MeshConfig tests ──────────────────────────────────────────────

    #[test]
    fn mesh_config_default_values() {
        let config = MeshConfig::default();
        assert_eq!(config.heartbeat_interval_secs, 5);
        assert_eq!(config.suspect_timeout_secs, 10);
        assert_eq!(config.dead_timeout_secs, 30);
    }

    #[test]
    fn mesh_config_serde_roundtrip() {
        let config = MeshConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let back: MeshConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, config);
    }

    #[test]
    fn mesh_config_deny_unknown_fields() {
        let json = r#"{"heartbeat_interval_secs":5,"suspect_timeout_secs":10,"dead_timeout_secs":30,"extra":1}"#;
        assert!(
            serde_json::from_str::<MeshConfig>(json).is_err(),
            "should reject unknown fields"
        );
    }

    // ── GossipConfig tests ────────────────────────────────────────────

    #[test]
    fn gossip_config_default_values() {
        let config = GossipConfig::default();
        assert_eq!(config.coordinator_interval_secs, 5);
    }

    #[test]
    fn gossip_config_serde_roundtrip() {
        let config = GossipConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let back: GossipConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, config);
    }

    #[test]
    fn gossip_config_deny_unknown_fields() {
        let json = r#"{"coordinator_interval_secs":5,"extra":1}"#;
        assert!(
            serde_json::from_str::<GossipConfig>(json).is_err(),
            "should reject unknown fields"
        );
    }
}
