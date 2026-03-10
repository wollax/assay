mod commands;

use clap::{CommandFactory, Parser, Subcommand};

use commands::{assay_dir, project_root};

#[derive(Parser)]
#[command(
    name = "assay",
    version,
    about = "Agentic development kit with spec-driven workflows",
    long_about = None,
    after_long_help = "\
Examples:
  Initialize a new project:
    assay init
    assay init --name my-project

  List and inspect specs:
    assay spec list
    assay spec show auth-flow
    assay spec show auth-flow --json

  Run quality gates:
    assay gate run auth-flow
    assay gate run auth-flow --verbose
    assay gate run auth-flow --json
    assay gate run --all

  Start the MCP server (for AI agent integration):
    assay mcp serve"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new Assay project in the current directory
    #[command(after_long_help = "\
Examples:
  Create a project using the directory name:
    assay init

  Create a project with a custom name:
    assay init --name my-project")]
    Init {
        /// Override the inferred project name
        #[arg(long)]
        name: Option<String>,
    },
    /// MCP server operations
    #[command(after_long_help = "\
Examples:
  Start the stdio MCP server:
    assay mcp serve")]
    Mcp {
        #[command(subcommand)]
        command: commands::mcp::McpCommand,
    },
    /// Manage spec files
    #[command(after_long_help = "\
Examples:
  List all specs in the project:
    assay spec list

  Show a spec's criteria as a table:
    assay spec show auth-flow

  Show a spec as JSON (for agent consumption):
    assay spec show auth-flow --json")]
    Spec {
        #[command(subcommand)]
        command: commands::spec::SpecCommand,
    },
    /// Manage quality gates
    #[command(after_long_help = "\
Examples:
  Run gates for a spec:
    assay gate run auth-flow

  Run with verbose output (show all evidence):
    assay gate run auth-flow --verbose

  Run with a custom timeout (seconds):
    assay gate run auth-flow --timeout 60

  Run and output JSON (for agent consumption):
    assay gate run auth-flow --json")]
    Gate {
        #[command(subcommand)]
        command: commands::gate::GateCommand,
    },
    /// Context window diagnostics for Claude Code sessions
    #[command(after_long_help = "\
Examples:
  Analyze the most recent session:
    assay context diagnose

  Analyze a specific session:
    assay context diagnose 3201041c-df85-4c91-a485-7b8c189f7636

  List sessions for the current project:
    assay context list

  List all sessions with token counts:
    assay context list --all --tokens

  Output diagnostics as JSON:
    assay context diagnose --json")]
    Context {
        #[command(subcommand)]
        command: commands::context::ContextCommand,
    },
    /// Manage git worktrees for spec isolation
    #[command(after_long_help = "\
Examples:
  Create an isolated worktree for a spec:
    assay worktree create auth-flow

  Create from a specific base branch:
    assay worktree create auth-flow --base develop

  List all active worktrees:
    assay worktree list

  Check worktree status:
    assay worktree status auth-flow

  Remove a worktree:
    assay worktree cleanup auth-flow

  Remove all worktrees:
    assay worktree cleanup --all --force")]
    Worktree {
        #[command(subcommand)]
        command: commands::worktree::WorktreeCommand,
    },
    /// Team state checkpointing
    #[command(after_long_help = "\
Examples:
  Take a snapshot of current team state:
    assay checkpoint save

  Take a snapshot with a custom trigger label:
    assay checkpoint save --trigger \"pre-deploy\"

  Show the latest checkpoint:
    assay checkpoint show

  Show latest as JSON:
    assay checkpoint show --json

  List recent checkpoints:
    assay checkpoint list

  List more checkpoints:
    assay checkpoint list --limit 25")]
    Checkpoint {
        #[command(subcommand)]
        command: commands::checkpoint::CheckpointCommand,
    },
}

/// Core CLI logic. Returns an exit code on success.
async fn run() -> anyhow::Result<i32> {
    let cli = Cli::try_parse().unwrap_or_else(|e| e.exit());

    match cli.command {
        Some(Command::Init { name }) => commands::init::handle_init(name),
        Some(Command::Mcp { command }) => commands::mcp::handle(command).await,
        Some(Command::Spec { command }) => commands::spec::handle(command),
        Some(Command::Gate { command }) => commands::gate::handle(command),
        Some(Command::Context { command }) => commands::context::handle(command),
        Some(Command::Worktree { command }) => commands::worktree::handle(command),
        Some(Command::Checkpoint { command }) => commands::checkpoint::handle(command),
        None => {
            // Note: project detection checks cwd only — no upward traversal.
            // Running `assay` from a subdirectory of a project shows the hint.
            let root = project_root()?;
            if assay_dir(&root).is_dir() {
                commands::init::show_status(&root)?;
                Ok(0)
            } else {
                eprintln!("Not an Assay project. Run `assay init` to get started.");
                if let Err(e) = Cli::command().print_help() {
                    eprintln!("Error: could not print help: {e}");
                }
                println!();
                Ok(1)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            1
        }
    };
    std::process::exit(code);
}
