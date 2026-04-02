# S03: Ratatui TUI + Server Config + Graceful Shutdown

**Goal:** Wire all S02 components into a working `smelt serve` CLI subcommand: load `ServerConfig` from a TOML file, start `dispatch_loop` + `DirectoryWatcher` + axum HTTP server + optional Ratatui TUI as concurrent tokio tasks, handle Ctrl+C with clean cancellation and teardown, and ship `examples/server.toml`.
**Demo:** `smelt serve --config examples/server.toml` starts, renders a live Ratatui table of jobs (status, phase, elapsed, attempt), accepts jobs via `POST /api/v1/jobs`, and shuts down cleanly on Ctrl+C with no orphaned containers — `cargo test --workspace` all green.

## Must-Haves

- `ServerConfig` parses correctly from TOML with all fields; validation rejects invalid config (port 0, max_concurrent 0, non-existent queue_dir parent)
- `smelt serve --config server.toml` starts and all four components run concurrently (dispatch_loop, DirectoryWatcher, axum HTTP, TUI)
- Ctrl+C cancels the parent CancellationToken, all running job tasks call teardown, dispatch_loop exits, TUI thread joins, process exits 0
- Ratatui TUI renders a live `Table` with one row per job (id, name, status, attempt, elapsed) refreshing every 250ms from `Arc<Mutex<ServerState>>`
- `q` key in TUI also triggers graceful shutdown (same code path as Ctrl+C)
- `--no-tui` flag disables TUI entirely; tracing output stays on stderr
- When TUI is active, tracing output is redirected to `.smelt/serve.log` (not stderr) so raw-mode display is clean
- `examples/server.toml` ships with documented inline comments for all fields
- `cargo test --workspace` all green; existing single-job tests unaffected

## Proof Level

- This slice proves: final-assembly (operational)
- Real runtime required: yes — `smelt serve` process starts and accepts HTTP requests in integration tests
- Human/UAT required: yes — live TUI rendering and Ctrl+C teardown with real Docker containers are verified in the UAT script

## Verification

```
# Unit tests — all in crates/smelt-cli/src/serve/config.rs and existing tests.rs
cargo test -p smelt-cli serve -- --nocapture

# Full workspace regression
cargo test --workspace

# Smoke: process starts and HTTP API responds
smelt serve --config examples/server.toml &
SERVER_PID=$!
sleep 2
curl -s http://127.0.0.1:8765/api/v1/jobs | jq  # expect []
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null
```

Integration tests (in `serve/tests.rs`):
- `test_server_config_roundtrip` — parse → validate a complete server.toml
- `test_server_config_missing_field_error` — missing queue_dir returns error
- `test_server_config_invalid_max_concurrent` — max_concurrent=0 returns validation error
- `test_serve_http_responds_while_running` — `smelt serve --no-tui --config <tmpfile>` starts, GET /api/v1/jobs returns [], process receives SIGTERM/cancel

## Observability / Diagnostics

- Runtime signals: `tracing::info!` on serve start ("smelt serve started on {host}:{port}"), dispatch_loop start/stop, TUI thread start/join; when `--no-tui` these appear on stderr; when TUI active they go to `.smelt/serve.log`
- Inspection surfaces: `GET /api/v1/jobs` (live job state, same as S02); `.smelt/serve.log` for tracing output in TUI mode; TUI table directly readable at terminal
- Failure visibility: `ServerConfig` validation errors surface before any component starts; TUI thread panic is caught, `ratatui::restore()` called, error printed to stderr; dispatch_loop exit logged
- Redaction constraints: no secrets in logs; `SMELT_LOG`/`RUST_LOG` env vars control log level

## Integration Closure

- Upstream surfaces consumed: `dispatch_loop(state, cancel_token, max_attempts)`, `DirectoryWatcher::new(queue_dir, state).watch()`, `build_router(state)` — all from S02; `run_with_cancellation()` from run.rs
- New wiring introduced in this slice: `ServerConfig` TOML struct + `smelt serve` CLI subcommand in `main.rs` that composes all four concurrent components; `serve/tui.rs` background thread; `serve/config.rs` config parser
- What remains before the milestone is truly usable end-to-end: nothing — this is the final assembly slice; R023 and R025 are validated here

## Tasks

- [x] **T01: ServerConfig TOML struct + examples/server.toml** `est:45m`
  - Why: `smelt serve` needs to read daemon-wide config from a TOML file; this is a pure data/parsing task with no runtime deps, independently verifiable, and blocks T02
  - Files: `crates/smelt-cli/src/serve/config.rs`, `crates/smelt-cli/src/serve/mod.rs`, `examples/server.toml`
  - Do: Create `serve/config.rs` with `ServerConfig` (queue_dir: PathBuf, max_concurrent: usize, retry_attempts: u32, retry_backoff_secs: u64, server: ServerNetworkConfig) and `ServerNetworkConfig` (host: String, port: u16) with serde defaults (retry_attempts=3, retry_backoff_secs=5, host="127.0.0.1", port=8765). Add `#[serde(deny_unknown_fields)]`. Add `ServerConfig::load(path: &Path) -> anyhow::Result<ServerConfig>` that reads the file, calls `toml::from_str`, then validates (max_concurrent > 0, port > 0). Add `pub(crate) mod config;` to `serve/mod.rs`. Write `examples/server.toml` with all fields and inline `#` comments explaining each. Add 3 unit tests to `serve/tests.rs` (roundtrip, missing field, invalid max_concurrent).
  - Verify: `cargo test -p smelt-cli serve::tests::test_server_config` — 3 tests pass; `cargo build -p smelt-cli` clean
  - Done when: `ServerConfig::load()` parses `examples/server.toml` without error in a test; invalid configs return descriptive `anyhow::Error`

- [x] **T02: `smelt serve` CLI subcommand wiring (no TUI)** `est:1h`
  - Why: Wires all S02 components (dispatch_loop, DirectoryWatcher, axum router) into the `smelt serve` entrypoint using `tokio::select!`; Ctrl+C handling; `--no-tui` flag; independently testable before T03 adds the TUI thread
  - Files: `crates/smelt-cli/src/main.rs`, `crates/smelt-cli/src/commands/mod.rs`, `crates/smelt-cli/src/commands/serve.rs`, `crates/smelt-cli/src/serve/mod.rs`
  - Do: Create `crates/smelt-cli/src/commands/serve.rs` with `ServeArgs { config: PathBuf, no_tui: bool }` and `execute(args) -> anyhow::Result<i32>`. Inside execute: load `ServerConfig`, create `Arc<Mutex<ServerState::new(config.max_concurrent)>>`, create parent `CancellationToken`. Run `tokio::fs::create_dir_all(config.queue_dir)` to ensure queue_dir exists. Bind `TcpListener::bind((config.server.host, config.server.port)).await?`. Use `tokio::select!` to race: `dispatch_loop(state, cancel_token.clone(), config.retry_attempts)`, `watcher.watch()` (constructed as `DirectoryWatcher::new(config.queue_dir, state)`), `axum::serve(listener, router)`, and `tokio::signal::ctrl_c()` → call `cancel_token.cancel()` + break. Wire `commands::serve` into `main.rs` `Commands` enum and match arm. Add `pub mod serve;` to `commands/mod.rs`. Add integration test `test_serve_http_responds_while_running` that starts serve with `--no-tui` and a temp config pointing to a real queue_dir; spawns a tokio task; asserts `GET /api/v1/jobs` returns 200 with `[]`; then drops/cancels.
  - Verify: `cargo test -p smelt-cli serve::tests::test_serve_http` — passes; `cargo build -p smelt-cli` — `smelt serve --help` works; `smelt serve --config examples/server.toml --no-tui` starts and `curl http://127.0.0.1:8765/api/v1/jobs` returns `[]`
  - Done when: `smelt serve --config examples/server.toml --no-tui` starts in under 2s, accepts HTTP requests, and exits cleanly on Ctrl+C

- [x] **T03: Ratatui TUI background thread** `est:1h`
  - Why: Closes R025 — live terminal dashboard for `smelt serve`; runs in a `std::thread::spawn` (not tokio::spawn) to avoid blocking the async runtime with crossterm's blocking `read()`
  - Files: `crates/smelt-cli/src/serve/tui.rs`, `crates/smelt-cli/src/serve/mod.rs`, `Cargo.toml` (workspace), `crates/smelt-cli/Cargo.toml`
  - Do: Add `ratatui = "0.29"` and `crossterm = "0.28"` to workspace `Cargo.toml` under `[workspace.dependencies]`; add both to `crates/smelt-cli/Cargo.toml` `[dependencies]`. Create `serve/tui.rs` with `pub(crate) fn run_tui(state: Arc<Mutex<ServerState>>, shutdown: Arc<AtomicBool>) -> std::thread::JoinHandle<()>`. Inside: call `ratatui::init()` which installs panic hook + enters alternate screen + raw mode. Loop calling `terminal.draw(|frame| render(frame, &state))` every 250ms. In `render()`: lock state briefly, clone jobs into a local Vec, release lock, build `Table` rows (id, manifest filename, status, attempt, elapsed). Poll `crossterm::event::poll(Duration::from_millis(0))` for key events: `q` or `Ctrl+C` sets `shutdown.store(true, Ordering::SeqCst)` and breaks. Also check `shutdown.load()` each iteration to exit when main wires shutdown. Call `ratatui::restore()` on exit. Add `pub(crate) mod tui;` to `serve/mod.rs`. Add `pub(crate) use tui::run_tui;`. Add `std::sync::atomic::AtomicBool` to the shutdown coordination. Add a unit test `test_tui_render_no_panic` that calls `render()` on a mock `Frame` with mock data — verifies the render function doesn't panic (use ratatui's `TestBackend` for this).
  - Verify: `cargo test -p smelt-cli serve::tests::test_tui_render_no_panic` — passes; `cargo build -p smelt-cli` clean with no ratatui/crossterm errors
  - Done when: `run_tui()` compiles, renders without panic on mock data, and the shutdown AtomicBool causes clean thread exit

- [x] **T04: Wire TUI + tracing redirect into smelt serve; cargo test --workspace green** `est:1h`
  - Why: Final assembly — integrates T03 TUI thread into T02's `execute()`, adds tracing-to-file redirect for TUI mode, validates end-to-end correctness, and ensures `cargo test --workspace` stays green
  - Files: `crates/smelt-cli/src/commands/serve.rs`, `crates/smelt-cli/src/main.rs`, `Cargo.toml` (workspace), `crates/smelt-cli/Cargo.toml`, `crates/smelt-cli/src/serve/tests.rs`
  - Do: Add `tracing-appender = "0.2"` to workspace `Cargo.toml` and `crates/smelt-cli/Cargo.toml` dependencies. In `main.rs`: when `Commands::Serve` is matched AND `!args.no_tui`, init the tracing subscriber before calling `execute()` using `tracing_appender::rolling::never(".smelt", "serve.log")` as the writer (create `.smelt/` dir first). When `--no-tui` or other subcommands: use the normal stderr path (existing logic). In `commands/serve.rs` `execute()`: create a `shutdown: Arc<AtomicBool> = Arc::new(AtomicBool::new(false))`. If `!args.no_tui`, call `run_tui(Arc::clone(&state), Arc::clone(&shutdown))` and store the `JoinHandle`. In the `tokio::select!` block, add a fourth arm: poll `shutdown.load(Ordering::SeqCst)` via `tokio::time::sleep(Duration::from_millis(100))` loop → when true, call `cancel_token.cancel()` and break. After `tokio::select!` exits: set `shutdown.store(true, Ordering::SeqCst)` to signal TUI to exit; join TUI thread with `handle.join().ok()`. Also set shutdown on Ctrl+C path. Fix any dead-code warnings by removing `#[allow(dead_code)]` now that `smelt serve` uses all S02 pub(crate) functions. Run `cargo test --workspace` and fix any failures.
  - Verify: `cargo test --workspace` — all tests pass (0 failures); `cargo build -p smelt-cli` — no warnings about unused serve functions; `smelt serve --config examples/server.toml` starts TUI in alternate screen, `smelt serve --config examples/server.toml --no-tui` logs to stderr
  - Done when: `cargo test --workspace` green; `smelt serve` starts with TUI and exits cleanly on `q` or Ctrl+C; `.smelt/serve.log` contains tracing output when TUI is active

## Observability / Diagnostics (aggregate)

- `.smelt/serve.log` — file-backed tracing log during TUI mode; survives process exit; `tail -f .smelt/serve.log` for real-time diagnostics
- `GET /api/v1/jobs` — authoritative runtime job state; readable during execution via `curl`
- `tracing::info!` at all lifecycle transitions: serve start, dispatch_loop start/stop, TUI start/stop, job start/complete/retry/fail
- TUI table — visual inspection surface showing all jobs; serves as the primary operator surface during a run
- Exit code 0 on clean shutdown; non-zero on config parse failure or port bind failure

## Files Likely Touched

- `crates/smelt-cli/src/serve/config.rs` (new)
- `crates/smelt-cli/src/serve/tui.rs` (new)
- `crates/smelt-cli/src/commands/serve.rs` (new)
- `crates/smelt-cli/src/serve/mod.rs`
- `crates/smelt-cli/src/commands/mod.rs`
- `crates/smelt-cli/src/main.rs`
- `crates/smelt-cli/src/serve/tests.rs`
- `crates/smelt-cli/Cargo.toml`
- `Cargo.toml`
- `examples/server.toml` (new)
