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

use crate::serve::dispatch::EventEnvConfig;
use crate::serve::events::AssayEvent;
use crate::serve::github::{GithubTrackerSource, SubprocessGhClient};
use crate::serve::linear::{LinearTrackerSource, ReqwestLinearClient};
use crate::serve::{
    AnyTrackerSource, DirectoryWatcher, ServerConfig, ServerState, SubprocessSshClient,
    TrackerPoller, build_router, dispatch_loop, http_api::resolve_auth, run_tui,
    tracker::load_template_manifest,
};

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

    let (event_bus, _event_rx) = tokio::sync::broadcast::channel::<AssayEvent>(1024);
    let state = Arc::new(Mutex::new(ServerState::load_or_new(
        config.queue_dir.clone(),
        config.max_concurrent,
        event_bus,
    )));
    let cancel_token = CancellationToken::new();
    let shutdown = Arc::new(AtomicBool::new(false));

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind {addr}: {e}"))?;

    // After binding, log the actual address (useful when port = 0 in tests).
    let bound_addr = listener.local_addr()?;
    tracing::info!("smelt serve started on {}", bound_addr);

    let resolved_auth = config.auth.as_ref().map(resolve_auth).transpose()?;

    // Detect host address for event env injection (D177).
    // Uses detect_host_address: SMELT_EVENT_HOST env override → macOS → Linux bridge.
    // Fallible: if Docker is unavailable (e.g. SSH-only dispatch), event injection
    // is disabled with a warning instead of failing the entire server startup.
    let event_env = match smelt_core::docker::DockerProvider::new() {
        Ok(docker_provider) => {
            match smelt_core::docker::detect_host_address(docker_provider.client()).await {
                Ok(host) => {
                    let write_token = resolved_auth.as_ref().map(|a| a.write_token.clone());
                    tracing::info!(
                        host = %host,
                        port = bound_addr.port(),
                        has_token = write_token.is_some(),
                        "computed event env config for container injection"
                    );
                    Some(EventEnvConfig {
                        host,
                        port: bound_addr.port(),
                        write_token,
                    })
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "failed to detect host address for event injection; \
                         set SMELT_EVENT_HOST to bypass Docker detection. \
                         Event env injection disabled for this session."
                    );
                    None
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Docker not available for host address detection; \
                 set SMELT_EVENT_HOST to bypass. \
                 Event env injection disabled for this session."
            );
            None
        }
    };

    if let Some(ref auth_cfg) = config.auth {
        tracing::info!(
            write_token_env = %auth_cfg.write_token_env,
            read_token_env = auth_cfg.read_token_env.as_deref().unwrap_or("<not configured>"),
            "auth enabled"
        );
    }

    let router = build_router(Arc::clone(&state), resolved_auth, cancel_token.clone());
    let watcher = DirectoryWatcher::new(config.queue_dir.clone(), Arc::clone(&state));
    let retry_attempts = config.retry_attempts;
    let workers = config.workers.clone();
    let ssh_timeout_secs = config.ssh_timeout_secs;

    // Build the optional tracker poller.
    let mut tracker_poller: Option<TrackerPoller> = match config.tracker {
        Some(ref tracker_config) => {
            let source = match tracker_config.provider.as_str() {
                "github" => {
                    let repo = tracker_config
                        .repo
                        .clone()
                        .expect("github provider requires repo (validated at startup)");
                    AnyTrackerSource::GitHub(GithubTrackerSource::new(
                        SubprocessGhClient,
                        repo,
                        tracker_config.label_prefix.clone(),
                    ))
                }
                "linear" => {
                    let api_key_env = tracker_config
                        .api_key_env
                        .as_deref()
                        .expect("linear provider requires api_key_env (validated at startup)");
                    let api_key = std::env::var(api_key_env).map_err(|_| {
                        anyhow::anyhow!(
                            "tracker api_key_env '{}' is not set in the environment",
                            api_key_env
                        )
                    })?;
                    let team_id = tracker_config
                        .team_id
                        .clone()
                        .expect("linear provider requires team_id (validated at startup)");
                    let client = ReqwestLinearClient::new(
                        api_key,
                        "https://api.linear.app/graphql".to_string(),
                    )?;
                    AnyTrackerSource::Linear(LinearTrackerSource::new(
                        client,
                        team_id,
                        tracker_config.label_prefix.clone(),
                    ))
                }
                other => {
                    anyhow::bail!(
                        "unsupported tracker provider '{}' (expected 'github' or 'linear')",
                        other
                    );
                }
            };

            let template_toml = std::fs::read_to_string(&tracker_config.manifest_template)?;
            let template = load_template_manifest(&tracker_config.manifest_template)?;
            let interval = Duration::from_secs(tracker_config.poll_interval_secs);

            tracing::info!(
                provider = %tracker_config.provider,
                poll_interval_secs = tracker_config.poll_interval_secs,
                "tracker poller configured"
            );

            Some(TrackerPoller {
                source,
                template,
                template_toml,
                config: tracker_config.clone(),
                state: Arc::clone(&state),
                cancel: cancel_token.child_token(),
                interval,
            })
        }
        None => None,
    };

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
        _ = dispatch_loop(Arc::clone(&state), cancel_token.clone(), retry_attempts, workers, SubprocessSshClient, ssh_timeout_secs, event_env) => {}
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
        // Tracker poller arm — runs only when [tracker] is configured.
        // When tracker is None, pending::<()>() never resolves so this arm
        // is effectively a no-op.
        result = async {
            match tracker_poller.as_mut() {
                Some(poller) => poller.run().await,
                None => { std::future::pending::<anyhow::Result<()>>().await }
            }
        } => {
            if let Err(e) = result {
                tracing::error!(error = %e, "tracker poller exited with error");
            }
            shutdown.store(true, Ordering::SeqCst);
            cancel_token.cancel();
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
