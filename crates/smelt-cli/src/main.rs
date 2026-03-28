//! Smelt CLI — containerized job execution engine.

use smelt_cli::commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "smelt",
    about = "Containerized job execution engine",
    propagate_version = true,
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a skeleton job manifest
    Init(commands::init::InitArgs),
    /// List past runs
    List(commands::list::ListArgs),
    /// Run a job manifest
    Run(commands::run::RunArgs),
    /// Start the job dispatch daemon
    Serve(commands::serve::ServeArgs),
    /// Show status of a running job
    Status(commands::status::StatusArgs),
    /// Watch a PR until merged or closed
    Watch(commands::watch::WatchArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Detect whether the user explicitly set a log filter via env var.
    // If set: use full format (timestamp, target, level) with the user-provided filter.
    // If not set: use bare-message format (no timestamp, no target, no level) with a
    // target-scoped default that shows info for smelt crates and warn for everything else.
    let explicit_env = std::env::var("SMELT_LOG").is_ok() || std::env::var("RUST_LOG").is_ok();

    let env_filter = tracing_subscriber::EnvFilter::try_from_env("SMELT_LOG")
        .or_else(|_| tracing_subscriber::EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new("smelt_cli=info,smelt_core=info,warn")
        });

    // When running `smelt serve` with TUI enabled, redirect tracing output to
    // `.smelt/serve.log` so that log lines don't corrupt the alternate-screen
    // terminal display.  TUI file appender always uses full format (levels are
    // useful in log files).  All other commands write to stderr.
    match &cli.command {
        Commands::Serve(args) if !args.no_tui => {
            std::fs::create_dir_all(".smelt").ok();
            let file_appender = tracing_appender::rolling::never(".smelt", "serve.log");
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(file_appender)
                .init();
        }
        _ if explicit_env => {
            // User explicitly set SMELT_LOG or RUST_LOG — full diagnostic format.
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .init();
        }
        _ => {
            // No env var set — bare-message format matching previous stderr behavior.
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .without_time()
                .with_target(false)
                .with_level(false)
                .with_writer(std::io::stderr)
                .init();
        }
    }

    let code = match cli.command {
        Commands::Init(ref args) => commands::init::execute(args).await,
        Commands::List(ref args) => commands::list::execute(args).await,
        Commands::Run(ref args) => commands::run::execute(args).await,
        Commands::Serve(ref args) => commands::serve::execute(args).await,
        Commands::Status(ref args) => commands::status::execute(args).await,
        Commands::Watch(ref args) => commands::watch::execute(args).await,
    };

    match code {
        Ok(c) => std::process::exit(c),
        Err(e) => {
            eprintln!("Error: {e:#}");
            std::process::exit(1);
        }
    }
}
