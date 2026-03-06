//! MCP (Model Context Protocol) server library for Assay.
//!
//! This crate provides the MCP server implementation that exposes Assay's
//! spec and gate operations as MCP tools. Five tools are available:
//!
//! - `spec_list` — discover available specs in the project
//! - `spec_get` — read a full spec definition by name
//! - `gate_run` — evaluate quality gate criteria (auto-creates sessions for agent criteria)
//! - `gate_report` — submit an agent evaluation for a criterion in an active session
//! - `gate_finalize` — finalize a session, persisting all evaluations as a GateRunRecord

mod server;

pub use server::{
    AssayServer, GateFinalizeParams, GateHistoryParams, GateReportParams, GateRunParams,
    SpecGetParams,
};

/// Re-export the `Parameters` wrapper from rmcp for use by integration tests.
pub use rmcp::handler::server::wrapper::Parameters;

/// Start the MCP server on stdio transport.
///
/// Delegates to the internal server module. Serves JSON-RPC on stdin/stdout
/// until the transport closes. Caller must initialize tracing before calling.
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    server::serve().await
}
