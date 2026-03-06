//! MCP (Model Context Protocol) server library for Assay.
//!
//! This crate provides the MCP server implementation that exposes Assay's
//! spec, gate, and context operations as MCP tools. Eight tools are available:
//!
//! - `spec_list` — discover available specs in the project
//! - `spec_get` — read a full spec definition by name
//! - `gate_run` — evaluate quality gate criteria (auto-creates sessions for agent criteria)
//! - `gate_report` — submit an agent evaluation for a criterion in an active session
//! - `gate_finalize` — finalize a session, persisting all evaluations as a GateRunRecord
//! - `gate_history` — query past gate run results for a spec
//! - `context_diagnose` — diagnose token usage and bloat in a Claude Code session
//! - `estimate_tokens` — estimate current token usage and context window health

mod server;

#[cfg(any(test, feature = "testing"))]
pub use server::{
    AssayServer, ContextDiagnoseParams, EstimateTokensParams, GateFinalizeParams,
    GateHistoryParams, GateReportParams, GateRunParams, SpecGetParams,
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
