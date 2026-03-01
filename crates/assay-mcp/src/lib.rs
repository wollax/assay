//! MCP (Model Context Protocol) server library for Assay.
//!
//! This crate provides the MCP server implementation that exposes Assay's
//! spec-driven workflows, quality gates, and reviews as MCP tools and resources.

mod spike;

/// Start the MCP server on stdio transport.
///
/// Delegates to the internal spike server. Serves JSON-RPC on stdin/stdout
/// until the transport closes. Caller must initialize tracing before calling.
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    spike::serve().await
}
