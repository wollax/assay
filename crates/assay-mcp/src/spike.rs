//! Spike MCP server for protocol validation.
//!
//! This is throwaway code — a GO/NO-GO gate proving rmcp 0.17 + stdio + Claude Code
//! can exchange JSON-RPC messages. Replaced entirely in Phase 8.

use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt, handler::server::tool::ToolRouter, model::*,
    tool, tool_handler, tool_router, transport::io::stdio,
};

/// Minimal MCP server with a single hardcoded tool for protocol validation.
#[derive(Clone)]
pub struct SpikeServer {
    tool_router: ToolRouter<Self>,
}

impl Default for SpikeServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl SpikeServer {
    /// Create a new spike server with the tool router initialized.
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Hardcoded greeting tool — zero user input, pure protocol validation.
    #[tool(description = "Spike: returns a hardcoded greeting to validate MCP protocol")]
    async fn spike_echo(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "spike: hello from assay",
        )]))
    }
}

#[tool_handler]
impl ServerHandler for SpikeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Assay MCP spike server. Single spike_echo tool for protocol validation."
                    .to_string(),
            ),
        }
    }
}

/// Start the spike MCP server on stdio transport.
///
/// Serves JSON-RPC on stdin/stdout until the transport closes.
/// Caller must initialize tracing before calling this function.
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting assay MCP server");

    let service = SpikeServer::new().serve(stdio()).await?;

    service.waiting().await?;
    Ok(())
}
