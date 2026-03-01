use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "assay",
    version,
    about = "Agentic development kit with spec-driven workflows"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// MCP server operations
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
}

#[derive(Subcommand)]
enum McpCommand {
    /// Start the MCP server (stdio transport)
    Serve,
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Mcp { command }) => match command {
            McpCommand::Serve => {
                init_mcp_tracing();
                if let Err(e) = assay_mcp::serve().await {
                    eprintln!("Error: {e:?}");
                    std::process::exit(1);
                }
            }
        },
        None => {
            println!("assay {}", env!("CARGO_PKG_VERSION"));
        }
    }
}
