//! MCP (Model Context Protocol) server library for Assay.
//!
//! This crate provides the MCP server implementation that exposes Assay's
//! spec-driven workflows, quality gates, and reviews as MCP tools and resources.

mod logging;
mod spike;

pub use spike::SpikeServer;

/// Start the MCP server on stdio transport.
///
/// Initializes tracing (stderr only), creates the spike server, and serves
/// JSON-RPC on stdin/stdout until the transport closes.
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    spike::serve().await
}
