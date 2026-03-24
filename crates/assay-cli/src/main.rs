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
    Spec {
        #[command(subcommand)]
        command: commands::spec::SpecCommand,
    },
    /// Manage quality gates
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
    /// Run a manifest through the end-to-end pipeline
    #[command(after_long_help = "\
Examples:
  Run a manifest:
    assay run manifest.toml

  Override timeout:
    assay run manifest.toml --timeout 900

  Output as JSON:
    assay run manifest.toml --json")]
    Run(commands::run::RunCommand),
    /// Agent harness configuration management
    #[command(after_long_help = "\
Examples:
  Generate Claude Code config:
    assay harness generate claude-code

  Install Codex config into project:
    assay harness install codex --spec auth-flow

  Check what would change:
    assay harness diff opencode")]
    Harness {
        #[command(subcommand)]
        command: commands::harness::HarnessCommand,
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
    /// Manage project milestones
    #[command(after_long_help = "\
Examples:
  List all milestones in the project:
    assay milestone list")]
    Milestone {
        #[command(subcommand)]
        command: commands::milestone::MilestoneCommand,
    },
    /// Run the guided authoring wizard to create a milestone and chunk specs.
    #[command(name = "plan", about = "Run the guided authoring wizard")]
    Plan,
    /// Analyse gate run history and milestone velocity
    #[command(after_long_help = "\
Examples:
  Show failure frequency and milestone velocity:
    assay history analytics

  Output analytics as JSON:
    assay history analytics --json")]
    History {
        #[command(subcommand)]
        command: commands::history::HistoryCommand,
    },
    /// Create a GitHub PR for a milestone after all chunk gates pass
    #[command(after_long_help = "\
Examples:
  Create a PR for a milestone (title defaults to 'feat: <milestone-slug>'):
    assay pr create my-feature

  Create a PR with a custom title and body:
    assay pr create my-feature --title 'feat: my feature' --body 'Implements the feature'")]
    Pr {
        #[command(subcommand)]
        command: commands::pr::PrCommand,
    },
}

/// Determine the tracing config based on the subcommand.
///
/// MCP serve uses `TracingConfig::mcp()` (warn level, no ANSI) because
/// stdout is reserved for JSON-RPC. All other subcommands use default (info).
fn tracing_config_for(command: &Option<Command>) -> assay_core::telemetry::TracingConfig {
    if let Some(Command::Mcp {
        command: commands::mcp::McpCommand::Serve,
    }) = command
    {
        assay_core::telemetry::TracingConfig::mcp()
    } else {
        assay_core::telemetry::TracingConfig::default()
    }
}

/// Core CLI logic. Returns an exit code on success.
async fn run() -> anyhow::Result<i32> {
    let cli = Cli::try_parse().unwrap_or_else(|e| e.exit());

    // Initialize tracing after argument parsing so MCP serve gets its own
    // config (warn level, no ANSI). The guard must live until process exit.
    let _tracing_guard = assay_core::telemetry::init_tracing(tracing_config_for(&cli.command));

    match cli.command {
        Some(Command::Init { name }) => commands::init::handle_init(name),
        Some(Command::Mcp { command }) => commands::mcp::handle(command).await,
        Some(Command::Spec { command }) => commands::spec::handle(command),
        Some(Command::Gate { command }) => commands::gate::handle(command),
        Some(Command::Context { command }) => commands::context::handle(command),
        Some(Command::Worktree { command }) => commands::worktree::handle(command),
        Some(Command::Run(cmd)) => commands::run::execute(&cmd),
        Some(Command::Harness { command }) => commands::harness::handle(command),
        Some(Command::Checkpoint { command }) => commands::checkpoint::handle(command),
        Some(Command::Milestone { command }) => commands::milestone::handle(command),
        Some(Command::Plan) => commands::plan::handle(),
        Some(Command::History { command }) => commands::history::handle(command),
        Some(Command::Pr { command }) => commands::pr::handle(command),
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
