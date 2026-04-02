---
id: T02
parent: S03
milestone: M006
provides:
  - ServeArgs struct (--config, --no-tui flags) in commands/serve.rs
  - execute() async fn wiring ServerConfig + dispatch_loop + DirectoryWatcher + axum::serve under tokio::select!
  - Ctrl+C handled via tokio::signal::ctrl_c() → CancellationToken::cancel()
  - Serve variant in Commands enum (main.rs)
  - dispatch_loop re-export via serve/mod.rs
  - ServerState re-export via serve/mod.rs
  - Integration test test_serve_http_responds_while_running (port 18765) passing
key_files:
  - crates/smelt-cli/src/commands/serve.rs
  - crates/smelt-cli/src/commands/mod.rs
  - crates/smelt-cli/src/main.rs
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - Used std::fs::create_dir_all (not tokio::fs) because tokio's fs feature is not enabled in Cargo.toml
  - Used fixed port 18765 for integration test instead of port-0 to avoid bound-address extraction across spawn boundary
  - Logged actual bound address (listener.local_addr()) rather than configured addr, so port-0 usage in tests shows real port
  - ServerState and dispatch_loop added as pub(crate) re-exports to serve/mod.rs for clean crate-level imports in commands/serve.rs
patterns_established:
  - tokio::select! with four arms: dispatch_loop, watcher.watch(), axum::serve, ctrl_c() — broadcast cancel on Ctrl+C
observability_surfaces:
  - tracing::info! on "smelt serve started on {addr}" (stderr in --no-tui mode)
  - tracing::info! on "Ctrl+C received — cancelling all jobs"
  - GET /api/v1/jobs responds from moment serve is up (live job state inspection)
  - Port bind failure surfaces as immediate anyhow error before tokio::select! loop starts
  - ServerConfig parse/validation errors surface before any component starts
duration: 20min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: `smelt serve` CLI subcommand wiring (no TUI)

**`smelt serve --config <file> --no-tui` wires dispatch_loop + DirectoryWatcher + axum HTTP server under tokio::select! with Ctrl+C cancellation — HTTP API verified responding via integration test.**

## What Happened

Created `crates/smelt-cli/src/commands/serve.rs` with `ServeArgs` (clap `--config` / `--no-tui`) and `execute()` that:
1. Loads and validates `ServerConfig` from the provided TOML path
2. Creates queue_dir if missing via `std::fs::create_dir_all`
3. Binds a `TcpListener` on `host:port` from config, logging the actual bound address
4. Runs `dispatch_loop`, `DirectoryWatcher::watch()`, `axum::serve()`, and `tokio::signal::ctrl_c()` concurrently under `tokio::select!`
5. On Ctrl+C: calls `cancel_token.cancel()` which broadcasts cancellation to all in-flight job tasks

Two re-exports were missing from `serve/mod.rs`: `ServerState` (from `queue`) and `dispatch_loop` (from `dispatch`) — both added as `pub(crate) use`.

`tokio::fs` feature is not enabled in the crate's Cargo.toml, so `std::fs::create_dir_all` was used instead.

The `Serve` variant was added to the `Commands` enum in `main.rs` with match arm calling `commands::serve::execute(args).await`.

Added `test_serve_http_responds_while_running` to `serve/tests.rs`: spawns `execute()` on port 18765, waits 500ms, does `GET /api/v1/jobs` via reqwest, asserts 200 + empty array, then aborts the handle.

## Verification

- `cargo build -p smelt-cli 2>&1 | grep "^error"` → no errors (only pre-existing `retry_backoff_secs` dead_code warning)
- `cargo test -p smelt-cli serve::tests::test_serve_http_responds_while_running` → 1 passed
- `cargo run -p smelt-cli -- serve --help` → shows `--config <CONFIG>` and `--no-tui` flags
- `cargo test --workspace` → 155 passed, 0 failed

## Diagnostics

- `GET http://127.0.0.1:8765/api/v1/jobs` → responds from moment serve is up
- stderr shows `tracing::info!` lines: "smelt serve started on …" and dispatch_loop start/stop
- Port bind failure → immediate `anyhow::Error` output before any component starts
- Config errors → immediate `anyhow::Error` output before port bind attempt

## Deviations

- Used `std::fs::create_dir_all` instead of `tokio::fs::create_dir_all` — tokio `fs` feature not enabled in Cargo.toml. Synchronous call is acceptable for a one-time startup dir creation.

## Known Issues

- Pre-existing dead_code warning on `retry_backoff_secs` field in `ServerConfig` — not introduced by this task; field will be used in T03/T04 for retry backoff logic.

## Files Created/Modified

- `crates/smelt-cli/src/commands/serve.rs` — new; ServeArgs + execute() wiring all components
- `crates/smelt-cli/src/commands/mod.rs` — added `pub mod serve;`
- `crates/smelt-cli/src/main.rs` — added `Serve(ServeArgs)` variant and match arm
- `crates/smelt-cli/src/serve/mod.rs` — added `pub(crate) use dispatch::dispatch_loop;` and `pub(crate) use queue::ServerState;`
- `crates/smelt-cli/src/serve/tests.rs` — added `test_serve_http_responds_while_running`
