---
estimated_steps: 6
estimated_files: 5
---

# T02: `smelt serve` CLI subcommand wiring (no TUI)

**Slice:** S03 — Ratatui TUI + Server Config + Graceful Shutdown
**Milestone:** M006

## Description

Wire all S02 components (dispatch_loop, DirectoryWatcher, axum HTTP server) and the T01 `ServerConfig` into the `smelt serve` CLI subcommand using `tokio::select!`. Adds Ctrl+C graceful shutdown via `CancellationToken::cancel()`. Includes `--no-tui` flag (always honoured in this task — TUI thread added in T03/T04). Independently verifiable: HTTP API responds and Ctrl+C exits cleanly before any TUI code exists.

## Steps

1. Create `crates/smelt-cli/src/commands/serve.rs` with:
   ```rust
   use clap::Parser;
   use std::path::PathBuf;
   use std::sync::{Arc, Mutex};
   use tokio_util::sync::CancellationToken;
   use tokio::net::TcpListener;
   use crate::serve::{DirectoryWatcher, ServerConfig, ServerState, build_router, dispatch_loop};

   #[derive(Parser, Debug)]
   pub struct ServeArgs {
       /// Path to the server configuration TOML file
       #[arg(long, short = 'c')]
       pub config: PathBuf,
       /// Disable the Ratatui TUI (tracing output stays on stderr)
       #[arg(long, default_value_t = false)]
       pub no_tui: bool,
   }

   pub async fn execute(args: &ServeArgs) -> anyhow::Result<i32> {
       let config = ServerConfig::load(&args.config)?;
       tokio::fs::create_dir_all(&config.queue_dir).await?;

       let state = Arc::new(Mutex::new(ServerState::new(config.max_concurrent)));
       let cancel_token = CancellationToken::new();

       let addr = format!("{}:{}", config.server.host, config.server.port);
       let listener = TcpListener::bind(&addr).await
           .map_err(|e| anyhow::anyhow!("Failed to bind {addr}: {e}"))?;
       tracing::info!("smelt serve started on {}", addr);

       let router = build_router(Arc::clone(&state));
       let watcher = DirectoryWatcher::new(config.queue_dir.clone(), Arc::clone(&state));

       tokio::select! {
           _ = dispatch_loop(Arc::clone(&state), cancel_token.clone(), config.retry_attempts) => {}
           _ = watcher.watch() => {}
           _ = axum::serve(listener, router) => {}
           _ = tokio::signal::ctrl_c() => {
               tracing::info!("Ctrl+C received — cancelling all jobs");
               cancel_token.cancel();
           }
       }

       Ok(0)
   }
   ```

2. Register `serve` in `crates/smelt-cli/src/commands/mod.rs`:
   - Add `pub mod serve;`

3. Wire into `crates/smelt-cli/src/main.rs`:
   - Import `commands::serve::ServeArgs`
   - Add `Serve(commands::serve::ServeArgs)` variant to `Commands` enum with doc comment `/// Start the job dispatch daemon`
   - Add match arm: `Commands::Serve(ref args) => commands::serve::execute(args).await.map_err(|e| anyhow::anyhow!(e))`
   - Note: `execute` returns `anyhow::Result<i32>` — the match arm must map correctly; adjust error handling to match the existing pattern (all other commands return `anyhow::Result<i32>` or `Result<i32, anyhow::Error>`)
   - Add `axum` import: `axum` is already in Cargo.toml; no new dep needed

4. Ensure `dispatch_loop` and `build_router` are re-exported from `serve/mod.rs` for use in `commands/serve.rs`:
   - `dispatch_loop` is already `pub(crate)` in `dispatch.rs`
   - `build_router` is already `pub(crate) use http_api::build_router` in `mod.rs`
   - `DirectoryWatcher` is already `pub(crate) use queue_watcher::DirectoryWatcher`
   - `ServerState` is in `queue.rs` as `pub struct` — already accessible within the crate
   - Add `pub(crate) use config::ServerConfig;` if not already done by T01
   - Add `pub(crate) use dispatch::dispatch_loop;` to `serve/mod.rs`

5. Add integration test `test_serve_http_responds_while_running` to `serve/tests.rs`:
   - Write a temp `server.toml` pointing to a temp `queue_dir` with `port = 0` (or a chosen free port like 18765)
   - Use `tokio::spawn` to run `execute(&ServeArgs { config: tmpfile, no_tui: true })` 
   - Sleep 500ms for startup
   - `GET http://127.0.0.1:<port>/api/v1/jobs` via reqwest; assert status 200 and body `[]`
   - Drop/abort the spawn handle to stop the server
   - Note: port 0 with TcpListener requires extracting the bound address — use a fixed high port (18765) for the test to keep it simple

6. Run `cargo build -p smelt-cli` and `cargo test -p smelt-cli serve::tests::test_serve` to confirm wiring works; run `cargo test --workspace` to check for regressions.

## Must-Haves

- [ ] `smelt serve --help` shows `--config <CONFIG>` and `--no-tui` flags
- [ ] `smelt serve --config examples/server.toml --no-tui` starts without error and `GET /api/v1/jobs` returns `[]`
- [ ] Ctrl+C (or cancellation) causes process exit 0 (no panic, no zombie processes)
- [ ] `cargo build -p smelt-cli` — no errors, no unused-import warnings
- [ ] Integration test `test_serve_starts_and_http_responds` passes: HTTP API responds 200 while serve is running

## Verification

- `cargo build -p smelt-cli 2>&1 | grep "^error"` → no errors
- `cargo test -p smelt-cli serve::tests::test_serve_http_responds_while_running` → test passes
- `smelt serve --help` → shows Serve subcommand with --config and --no-tui flags
- `cargo test --workspace` → all tests pass

## Observability Impact

- Signals added/changed: `tracing::info!` on serve start ("smelt serve started on {addr}"), Ctrl+C received — these appear on stderr in `--no-tui` mode; in T04 they'll be redirected to file when TUI is active
- How a future agent inspects this: `GET /api/v1/jobs` responds from the moment serve is up; `tracing::info!` confirms each component started; stderr shows config errors before any component starts
- Failure state exposed: port bind failure surfaces as immediate error before the tokio::select! loop; config parse errors surface before any component starts; Ctrl+C cancellation logged

## Inputs

- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig::load()` from T01
- `crates/smelt-cli/src/serve/mod.rs` — `dispatch_loop`, `DirectoryWatcher`, `build_router`, `ServerState` re-exports from S02
- `crates/smelt-cli/src/commands/mod.rs` — existing module registry
- `crates/smelt-cli/src/main.rs` — existing CLI entry point with Commands enum

## Expected Output

- `crates/smelt-cli/src/commands/serve.rs` — new file with `ServeArgs` + `execute()` function wiring all four components
- `crates/smelt-cli/src/commands/mod.rs` — `pub mod serve;` added
- `crates/smelt-cli/src/main.rs` — `Serve` variant in Commands enum + match arm
- `crates/smelt-cli/src/serve/mod.rs` — `pub(crate) use dispatch::dispatch_loop;` added if missing
- `crates/smelt-cli/src/serve/tests.rs` — `test_serve_starts_and_http_responds` test added
