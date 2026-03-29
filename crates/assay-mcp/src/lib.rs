//! MCP (Model Context Protocol) server library for Assay.
//!
//! This crate provides the MCP server implementation that exposes Assay's
//! spec, gate, context, worktree, and session operations as MCP tools.
//! Eighteen tools are available:
//!
//! - `spec_list` — discover available specs in the project
//! - `spec_get` — read a full spec definition by name
//! - `spec_validate` — statically validate a spec without running it
//! - `gate_run` — evaluate quality gate criteria (auto-creates sessions for agent criteria)
//! - `gate_report` — submit an agent evaluation for a criterion in an active session
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

mod server;
pub mod signal_server;

#[cfg(any(test, feature = "testing"))]
pub use server::{
    AssayServer, ContextDiagnoseParams, EstimateTokensParams, GateFinalizeParams,
    GateHistoryParams, GateReportParams, GateRunParams, MergeCheckParams, OrchestrateRunParams,
    OrchestrateStatusParams, SessionCreateParams, SessionGetParams, SessionListParams,
    SessionUpdateParams, SpecGetParams, SpecValidateParams,
};

#[cfg(any(test, feature = "testing"))]
pub use rmcp::handler::server::wrapper::Parameters;

/// Start the MCP server on stdio transport.
///
/// Delegates to the internal server module. Serves JSON-RPC on stdin/stdout
/// until the transport closes. Caller must initialize tracing before calling.
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    server::serve().await
}
