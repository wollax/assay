//! `smelt serve` subcommand — wire all S02/S03 components together.
//!
//! Binds an axum HTTP server, starts a `DirectoryWatcher`, and runs `dispatch_loop`
//! concurrently under a `tokio::select!`. Ctrl+C broadcasts cancellation to all
//! in-flight jobs via `CancellationToken::cancel()`.
//!
//! When `--no-tui` is NOT set, a Ratatui TUI thread is spawned on a dedicated
//! `std::thread` (keeping crossterm blocking I/O off the async runtime). An
//! `Arc<AtomicBool>` coordinates shutdown in both directions:
//!   - tokio side sets it to `true` when Ctrl+C / component exit occurs → TUI exits
//!   - TUI side sets it to `true` when user presses `q` / Ctrl+C → tokio exits

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::Parser;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::serve::{DirectoryWatcher, ServerConfig, ServerState, build_router, dispatch_loop, run_tui};

/// `smelt serve` command arguments.
#[derive(Parser, Debug)]
pub struct ServeArgs {
    /// Path to the server configuration TOML file
    #[arg(long, short = 'c')]
    pub config: PathBuf,
    /// Disable the Ratatui TUI (tracing output stays on stderr)
    #[arg(long, default_value_t = false)]
    pub no_tui: bool,
}

/// Entry point for `smelt serve`.
///
/// Loads `ServerConfig`, creates the queue directory if absent, then concurrently
/// runs up to five components under `tokio::select!`:
/// - `dispatch_loop` — polls `ServerState` and spawns job tasks
/// - `DirectoryWatcher::watch` — picks up manifest files from `queue_dir`
/// - `axum::serve` — HTTP API on `host:port`
/// - Ctrl+C handler — cancels all in-flight jobs
/// - TUI shutdown poller — signals tokio when user presses `q` in the TUI
///
/// When TUI mode is active, a `std::thread` runs the Ratatui event loop and the
/// `Arc<AtomicBool>` coordinates clean teardown from either side.
pub async fn execute(args: &ServeArgs) -> anyhow::Result<i32> {
    let config = ServerConfig::load(&args.config)?;
    std::fs::create_dir_all(&config.queue_dir)?;

    let state = Arc::new(Mutex::new(ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)));
    let cancel_token = CancellationToken::new();
    let shutdown = Arc::new(AtomicBool::new(false));

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind {addr}: {e}"))?;

    // After binding, log the actual address (useful when port = 0 in tests).
    let bound_addr = listener.local_addr()?;
    tracing::info!("smelt serve started on {}", bound_addr);

    let router = build_router(Arc::clone(&state));
    let watcher = DirectoryWatcher::new(config.queue_dir.clone(), Arc::clone(&state));
    let retry_attempts = config.retry_attempts;

    // Spawn the TUI thread when TUI mode is active.
    let tui_handle: Option<std::thread::JoinHandle<()>> = if !args.no_tui {
        Some(run_tui(Arc::clone(&state), Arc::clone(&shutdown)))
    } else {
        None
    };

    // Clone for the shutdown-poll async block.
    let shutdown_poll = Arc::clone(&shutdown);
    let cancel_for_tui = cancel_token.clone();

    tokio::select! {
        _ = dispatch_loop(Arc::clone(&state), cancel_token.clone(), retry_attempts) => {}
        _ = watcher.watch() => {}
        _ = axum::serve(listener, router) => {}
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Ctrl+C received — cancelling all jobs");
            shutdown.store(true, Ordering::SeqCst);
            cancel_token.cancel();
        }
        // Poll the TUI shutdown flag every 100 ms. When the user presses `q`
        // inside the TUI, this arm fires and cancels the tokio runtime.
        _ = async move {
            loop {
                if shutdown_poll.load(Ordering::SeqCst) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        } => {
            tracing::info!("TUI requested shutdown");
            cancel_for_tui.cancel();
        }
    }

    // Signal the TUI thread to exit (in case tokio exited first).
    shutdown.store(true, Ordering::SeqCst);

    // Wait for the TUI thread to finish and restore the terminal.
    if let Some(handle) = tui_handle {
        handle.join().ok();
    }

    Ok(0)
}
