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
            init_mcp_tracing();
            assay_mcp::serve()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(0)
        }
    }
}

/// Initialize tracing to stderr for MCP server operation.
///
/// Default level is `warn`. Override via `RUST_LOG` environment variable.
/// Stdout is reserved for JSON-RPC — all diagnostics go to stderr.
fn init_mcp_tracing() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    if let Err(e) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init()
    {
        eprintln!("[assay] warning: failed to initialize tracing: {e}");
    }
}
