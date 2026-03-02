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
    /// Initialize a new Assay project in the current directory
    Init {
        /// Override the inferred project name
        #[arg(long)]
        name: Option<String>,
    },
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
        Some(Command::Init { name }) => {
            let root = std::env::current_dir().unwrap_or_else(|e| {
                eprintln!("Error: could not determine current directory: {e}");
                std::process::exit(1);
            });
            let options = assay_core::init::InitOptions { name };
            match assay_core::init::init(&root, &options) {
                Ok(result) => {
                    println!("  Created assay project `{}`", result.project_name);
                    for path in &result.created_files {
                        let display = path.strip_prefix(&root).unwrap_or(path);
                        println!("    created {}", display.display());
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
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
