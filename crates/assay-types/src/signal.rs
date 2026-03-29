//! Types for the Assay signal endpoint.
//!
//! These types define the HTTP API surface for cross-job signaling.
//! Smelt (or any external orchestrator) POSTs a [`SignalRequest`] to
//! `/api/v1/signal` to route a [`PeerUpdate`] into a named session's inbox.
//! `GET /api/v1/state` returns [`AssayServerState`].

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema_registry;

// ── Gate summary (lightweight) ──────────────────────────────────────

/// Lightweight gate pass/fail/skip counts included in a [`PeerUpdate`].
///
/// This is a signal-specific summary — not the same as the full gate types
/// in [`crate::gate_run`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GateSummary {
    /// Number of gate criteria that passed.
    pub passed: u32,
    /// Number of gate criteria that failed.
    pub failed: u32,
    /// Number of gate criteria that were skipped.
    pub skipped: u32,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "signal-gate-summary",
        generate: || schemars::schema_for!(GateSummary),
    }
}

// ── PeerUpdate ──────────────────────────────────────────────────────

/// A cross-job status update sent from one Smelt job to another via the
/// Assay signal endpoint.
///
/// Smelt's `notify` section triggers a POST of this payload to
/// `POST /api/v1/signal` when a source session completes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PeerUpdate {
    /// Smelt job identifier of the sending job.
    pub source_job: String,
    /// Session name within the source job that produced this update.
    pub source_session: String,
    /// Files changed by the source session (relative paths).
    pub changed_files: Vec<String>,
    /// Lightweight gate result summary from the source session.
    pub gate_summary: GateSummary,
    /// Git branch the source session worked on.
    pub branch: String,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "peer-update",
        generate: || schemars::schema_for!(PeerUpdate),
    }
}

// ── SignalRequest ───────────────────────────────────────────────────

/// Envelope for routing a [`PeerUpdate`] to a specific session.
///
/// Posted to `POST /api/v1/signal` on the Assay signal endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SignalRequest {
    /// Name of the target session that should receive this update.
    pub target_session: String,
    /// The peer update payload to deliver.
    pub update: PeerUpdate,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "signal-request",
        generate: || schemars::schema_for!(SignalRequest),
    }
}

// ── RunSummary ──────────────────────────────────────────────────────

/// Summary of a single active orchestrator run, reported in
/// [`AssayServerState`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunSummary {
    /// Unique run identifier.
    pub run_id: String,
    /// Spec name being executed.
    pub spec_name: String,
    /// Number of sessions in this run.
    pub session_count: u32,
    /// When the run started (UTC).
    pub started_at: chrono::DateTime<chrono::Utc>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "run-summary",
        generate: || schemars::schema_for!(RunSummary),
    }
}

// ── PollSignalsResult ───────────────────────────────────────────────

/// Result type for the `poll_signals` MCP tool.
///
/// Wraps a vector of [`PeerUpdate`] messages read from a session's inbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PollSignalsResult {
    /// Signals consumed from the session inbox (exactly-once delivery).
    pub signals: Vec<PeerUpdate>,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "poll-signals-result",
        generate: || schemars::schema_for!(PollSignalsResult),
    }
}

// ── AssayServerState ────────────────────────────────────────────────

/// Global server state returned by `GET /api/v1/state`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AssayServerState {
    /// Currently active orchestrator runs.
    pub active_runs: Vec<RunSummary>,
    /// Seconds since the MCP server started.
    pub uptime_secs: u64,
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "assay-server-state",
        generate: || schemars::schema_for!(AssayServerState),
    }
}
