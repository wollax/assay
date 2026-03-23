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
    /// Show status of a running job
    Status(commands::status::StatusArgs),
    /// Watch a PR until merged or closed
    Watch(commands::watch::WatchArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize tracing subscriber
    let env_filter = tracing_subscriber::EnvFilter::try_from_env("SMELT_LOG")
        .or_else(|_| tracing_subscriber::EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    let code = match cli.command {
        Commands::Init(ref args) => commands::init::execute(args).await,
        Commands::List(ref args) => commands::list::execute(args).await,
        Commands::Run(ref args) => commands::run::execute(args).await,
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
