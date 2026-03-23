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

    // Build the env filter from SMELT_LOG or RUST_LOG, defaulting to "warn".
    let env_filter = tracing_subscriber::EnvFilter::try_from_env("SMELT_LOG")
        .or_else(|_| tracing_subscriber::EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));

    // When running `smelt serve` with TUI enabled, redirect tracing output to
    // `.smelt/serve.log` so that log lines don't corrupt the alternate-screen
    // terminal display.  All other commands write to stderr as usual.
    match &cli.command {
        Commands::Serve(args) if !args.no_tui => {
            std::fs::create_dir_all(".smelt").ok();
            let file_appender = tracing_appender::rolling::never(".smelt", "serve.log");
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(file_appender)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
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
