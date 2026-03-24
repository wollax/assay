use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum McpCommand {
    /// Start the MCP server (stdio transport)
    #[command(after_long_help = "\
Examples:
  Start the server for Claude Code integration:
    assay mcp serve

  Start with debug logging:
    RUST_LOG=debug assay mcp serve")]
    Serve,
}

/// Handle MCP subcommands.
pub(crate) async fn handle(command: McpCommand) -> anyhow::Result<i32> {
    match command {
        McpCommand::Serve => {
            // Tracing is initialized centrally in main() with TracingConfig::mcp()
            // when the subcommand is `mcp serve`. No per-command init needed.
            assay_mcp::serve()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(0)
        }
    }
}
