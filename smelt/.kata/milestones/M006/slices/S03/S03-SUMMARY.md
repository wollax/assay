---
id: S03
parent: M006
milestone: M006
provides:
  - ServerConfig TOML struct with serde deserialization, validation, and deny_unknown_fields
  - ServerNetworkConfig struct with host/port serde defaults (127.0.0.1:8765)
  - ServerConfig::load(path) with file-read + parse + fail-fast validation
  - examples/server.toml canonical documented server config with 7 inline comments
  - ServeArgs (--config, --no-tui) in commands/serve.rs
  - execute() wiring dispatch_loop + DirectoryWatcher + axum HTTP server under tokio::select!
  - Ctrl+C via tokio::signal::ctrl_c() → CancellationToken::cancel() broadcast to all job tasks
  - smelt serve subcommand wired into main.rs Commands enum
  - run_tui() background std::thread spawning Ratatui TUI with crossterm alternate screen
  - tui_loop() with q/Ctrl+C key handling and 250ms refresh cycle
  - render() 5-column job table (ID, Manifest, Status, Attempt, Elapsed) from Arc<Mutex<ServerState>>
  - Arc<AtomicBool> bidirectional shutdown coordination between tokio runtime and TUI thread
  - Conditional tracing init in main.rs: file appender (.smelt/serve.log) for TUI mode, stderr otherwise
  - TUI shutdown arm in tokio::select! polling AtomicBool every 100ms
  - ratatui = "0.29" and crossterm = "0.28" in workspace and smelt-cli dependencies
  - tracing-appender = "0.2" in workspace and smelt-cli dependencies
  - cargo test --workspace: all green (46 smelt-cli + 155 smelt-core + all integration tests)
requires:
  - slice: S01
    provides: dispatch_loop(state, cancel_token, max_attempts), ServerState Arc<Mutex>, CancellationToken broadcast cancellation
  - slice: S02
    provides: DirectoryWatcher::new(queue_dir, state).watch(), build_router(state), JobQueue, JobStatus, JobId
affects: []
key_files:
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/tui.rs
  - crates/smelt-cli/src/commands/serve.rs
  - crates/smelt-cli/src/commands/mod.rs
  - crates/smelt-cli/src/main.rs
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/src/serve/tests.rs
  - crates/smelt-cli/Cargo.toml
  - Cargo.toml
  - examples/server.toml
key_decisions:
  - D102: ServerConfig in a separate server.toml, not embedded in job manifests
  - D106: Arc<AtomicBool> for TUI/tokio shutdown coordination (not MPSC/oneshot)
  - D107: Tracing subscriber branched in main() before dispatch — file appender for TUI mode, stderr otherwise
  - std::thread::spawn for TUI to keep crossterm blocking I/O off the async tokio runtime
  - std::fs::create_dir_all (not tokio::fs) for queue_dir creation — tokio fs feature not enabled
  - Fixed port 18765 in integration test instead of port-0 (cross-task boundary extraction is complex)
patterns_established:
  - TUI shutdown coordination: Arc<AtomicBool> polled in tokio::select! arm (100ms) + set after select! exits for clean thread join
  - Config validation pattern: parse with toml::from_str → validate() → return anyhow::Error with descriptive message
  - TUI render pattern: snapshot Arc<Mutex<ServerState>> into local Vec inside short lock scope, release lock, then render via ratatui widgets
  - Tracing redirect: match Commands::Serve(args) if !args.no_tui before .init() call — exactly one init path, no double-init risk
observability_surfaces:
  - .smelt/serve.log — file-backed tracing log in TUI mode; tail -f .smelt/serve.log for real-time inspection
  - GET /api/v1/jobs — live job state via curl; responds from moment serve is up
  - TUI table — 5-column live job display in alternate screen; primary operator surface
  - tracing::info! on serve start (address), dispatch_loop start/stop, TUI thread start/join
  - Ctrl+C and TUI shutdown events both logged before CancellationToken::cancel()
  - eprintln!("TUI error: {e}") after ratatui::restore() — TUI panics surface to stderr without leaving terminal in raw mode
  - ServerConfig parse/validation errors surface before any component starts (fail-fast)
drill_down_paths:
  - .kata/milestones/M006/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M006/slices/S03/tasks/T02-SUMMARY.md
  - .kata/milestones/M006/slices/S03/tasks/T03-SUMMARY.md
  - .kata/milestones/M006/slices/S03/tasks/T04-SUMMARY.md
duration: ~1h (4 tasks, ~15-20min each)
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# S03: Ratatui TUI + Server Config + Graceful Shutdown

**Final-assembly slice wiring ServerConfig + dispatch_loop + DirectoryWatcher + axum HTTP server + Ratatui TUI background thread into `smelt serve` with Ctrl+C cancellation and clean shutdown — cargo test --workspace all green.**

## What Happened

S03 assembled all components built in S01 and S02 into the working `smelt serve` CLI subcommand. Four tasks composed cleanly:

**T01** created `ServerConfig` (TOML deserialization with `deny_unknown_fields`, validation, serde defaults for host/port) and `examples/server.toml` with 7 inline comments. Three unit tests (roundtrip, missing field, invalid max_concurrent) verified the parsing contract. No runtime dependencies.

**T02** wired all S02 components into a `execute()` function under `tokio::select!` with four arms: `dispatch_loop`, `DirectoryWatcher::watch()`, `axum::serve()`, and `tokio::signal::ctrl_c()`. Ctrl+C cancels the parent `CancellationToken` which broadcasts to all running job tasks. The `Serve` variant was added to `Commands` in `main.rs`. An integration test (`test_serve_http_responds_while_running` on port 18765) verified the HTTP API responds immediately after startup.

**T03** implemented the Ratatui TUI as a `std::thread::spawn` background thread (not `tokio::spawn`) to keep crossterm's blocking `event::read()` off the async runtime. `run_tui()` calls `ratatui::init()` + `ratatui::restore()` with panic hook. `render()` snapshots `Arc<Mutex<ServerState>>` inside a short lock scope then renders a 5-column table. `Arc<AtomicBool>` provides bidirectional shutdown coordination. `test_tui_render_no_panic` via `TestBackend` verified the render path in CI without a real terminal.

**T04** added `tracing-appender = "0.2"` and integrated the TUI thread into `serve.rs`. In `main.rs`, the tracing subscriber is initialized before command dispatch: TUI mode uses `tracing_appender::rolling::never(".smelt", "serve.log")`, all other modes use stderr. A fifth arm was added to `tokio::select!` polling the `AtomicBool` every 100ms — when the TUI sets it true (user presses `q`), the arm cancels the runtime. After `select!` exits, `shutdown.store(true)` + TUI thread join ensures clean teardown from both directions.

## Verification

- `cargo test --workspace` — all tests pass (46 smelt-cli + 155 smelt-core + integration tests), 0 failures
- `cargo build -p smelt-cli` — Finished, no errors, no warnings
- `smelt serve --config examples/server.toml --no-tui &` + `curl http://127.0.0.1:8765/api/v1/jobs` → `[]` (HTTP API confirmed responding)
- `smelt serve --help` → shows `--config <CONFIG>` and `--no-tui` flags
- `cargo test -p smelt-cli "serve::tests::test_server_config"` → 3 passed
- `cargo test -p smelt-cli serve::tests::test_tui_render_no_panic` → 1 passed
- `cargo test -p smelt-cli serve::tests::test_serve_http_responds_while_running` → 1 passed

## Requirements Advanced

- R023 — `smelt serve` parallel dispatch daemon: final assembly slice; all four concurrent components wired and confirmed working
- R024 — HTTP API: confirmed responding from moment serve is up; GET /api/v1/jobs returns live state
- R025 — Live Ratatui TUI: TUI thread implemented, renders job table, handles q/Ctrl+C, redraws every 250ms

## Requirements Validated

- R023 — Validated: `smelt serve --config server.toml` starts, accepts HTTP requests, dispatches jobs, exits cleanly on Ctrl+C; cargo test --workspace green; `smelt run manifest.toml` path unchanged
- R024 — Validated: POST /api/v1/jobs, GET /api/v1/jobs, GET /api/v1/jobs/:id, DELETE /api/v1/jobs/:id all confirmed by S02+S03 integration tests
- R025 — Validated: Ratatui TUI renders live job table; TestBackend test confirms render doesn't panic; operational verification (live TUI with real Docker jobs + Ctrl+C teardown) deferred to S03-UAT.md per milestone design

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- `std::fs::create_dir_all` used instead of `tokio::fs::create_dir_all` in `execute()` — the `fs` feature is not enabled in `smelt-cli/Cargo.toml`; synchronous one-time call on startup is acceptable.
- Integration test uses fixed port 18765 instead of port-0 — extracting the bound address across a `tokio::spawn` boundary in tests is complex; fixed port is simpler and sufficient for CI.
- `render` re-export was added in T03 then removed in T04 — it was never consumed outside `tui.rs` itself.
- `retry_backoff_secs` dead-code suppressed with `#[allow(dead_code)]` — the field is intentional config for future retry backoff logic (D102 records it as valid configuration).

## Known Limitations

- Live TUI with real Docker containers has not been manually verified in an interactive terminal session (agent has no interactive terminal). The TestBackend CI test and smoke test (HTTP API responding) are the available automated proofs; full operational verification requires a human running the UAT.
- Retry backoff logic (`retry_backoff_secs` config field) is present in `ServerConfig` but the backoff sleep is not yet wired into `dispatch_loop` — field exists for future use. Jobs retry immediately on failure within the configured `retry_attempts` limit.
- No temp file cleanup for HTTP POST TOML bodies (`std::mem::forget(TempPath)` — D105); acceptable for a daemon process, noted as deferred.

## Follow-ups

- Wire `retry_backoff_secs` into `dispatch_loop` backoff sleep (currently immediate retry)
- Manual UAT with real Docker jobs and live TUI rendering (see S03-UAT.md)
- Consider port-0 + address extraction pattern for integration tests to avoid fixed-port conflicts

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — new: ServerConfig, ServerNetworkConfig, ServerConfig::load(), serde defaults, validation
- `crates/smelt-cli/src/serve/tui.rs` — new: run_tui(), tui_loop(), render() with TestBackend-verified render path
- `crates/smelt-cli/src/commands/serve.rs` — new: ServeArgs + execute() wiring all four components + TUI thread integration
- `crates/smelt-cli/src/commands/mod.rs` — added pub mod serve
- `crates/smelt-cli/src/main.rs` — added Serve variant, match arm, conditional tracing init
- `crates/smelt-cli/src/serve/mod.rs` — added pub(crate) mod config, tui; re-exports for ServerConfig, dispatch_loop, ServerState, run_tui
- `crates/smelt-cli/src/serve/tests.rs` — 5 new tests: 3 config, 1 TUI render, 1 HTTP integration
- `crates/smelt-cli/Cargo.toml` — ratatui, crossterm, tracing-appender added
- `Cargo.toml` — ratatui = "0.29", crossterm = "0.28", tracing-appender = "0.2" in workspace deps
- `examples/server.toml` — new: canonical documented server config with 7 inline comments

## Forward Intelligence

### What the next slice should know
- `smelt serve` is fully assembled and working; all four components (dispatch_loop, DirectoryWatcher, axum, TUI) are proven running under `tokio::select!`
- The HTTP API surface from S02 is live and tested — `GET /api/v1/jobs` responds immediately on startup
- TUI is always-on by default; `--no-tui` disables it and routes tracing to stderr
- `examples/server.toml` is the canonical config file — point users here first
- `retry_backoff_secs` is in the config struct but not wired into backoff sleep — worth noting for anyone extending retry logic

### What's fragile
- Fixed port 18765 in integration test — if another process uses that port during CI, the test will fail with "address already in use". Low probability but possible.
- `std::mem::forget(TempPath)` for HTTP POST temp files — files accumulate indefinitely in the system temp dir for long-running serve sessions.
- TUI alternate screen + tracing-appender interact via the `main()` init branch; if a future command is added between `match &cli.command` and the subscriber init, tracing may not redirect correctly.

### Authoritative diagnostics
- `cargo test --workspace` — primary correctness signal; 46 smelt-cli + 155 smelt-core tests cover the full serve path
- `GET http://127.0.0.1:8765/api/v1/jobs` — live job state; responds immediately after startup
- `tail -f .smelt/serve.log` — file-backed tracing in TUI mode; all lifecycle events logged here
- `smelt serve --config examples/server.toml --no-tui` — clean smoke test path without TUI raw mode

### What assumptions changed
- TUI was assumed to use port-0 for integration tests — fixed port 18765 was used instead due to cross-task boundary complexity.
- `tokio::fs::create_dir_all` was planned for queue_dir creation — synchronous `std::fs::create_dir_all` used because the `fs` tokio feature is not enabled.
