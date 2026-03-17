//! Smelt CLI — containerized job execution engine.

mod commands;

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
    /// Run a job manifest
    Run(commands::run::RunArgs),
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
        Commands::Run(ref args) => commands::run::execute(args).await,
    };

    match code {
        Ok(c) => std::process::exit(c),
        Err(e) => {
            eprintln!("Error: {e:#}");
            std::process::exit(1);
        }
    }
}
