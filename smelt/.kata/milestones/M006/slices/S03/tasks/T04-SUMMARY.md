---
id: T04
parent: S03
milestone: M006
provides:
  - "tracing-appender = 0.2 added to workspace and smelt-cli dependencies"
  - "main.rs conditionally inits tracing: file appender (.smelt/serve.log) for TUI mode, stderr for all other modes"
  - "commands/serve.rs integrates run_tui() with Arc<AtomicBool> shutdown coordination"
  - "TUI shutdown arm in tokio::select! polls AtomicBool every 100ms and cancels the runtime"
  - "serve/mod.rs exports only run_tui (unused render re-export removed)"
  - "cargo test --workspace: 46 smelt-cli unit + integration tests pass, 0 failures"
key_files:
  - "crates/smelt-cli/src/main.rs"
  - "crates/smelt-cli/src/commands/serve.rs"
  - "crates/smelt-cli/src/serve/mod.rs"
  - "crates/smelt-cli/src/serve/config.rs"
  - "Cargo.toml"
  - "crates/smelt-cli/Cargo.toml"
key_decisions:
  - "Placed tracing init conditional in main() before execute() dispatch — avoids double-init and keeps routing co-located with command match"
  - "Used Arc<AtomicBool> clone named shutdown_poll for the tokio select arm to avoid borrow conflicts across the select! macro"
  - "Added #[allow(dead_code)] to retry_backoff_secs field with comment — it is intentional config for future dispatch retry logic, not an oversight"
  - "Removed re-export of render from serve/mod.rs — it was unused outside the tui module"
patterns_established:
  - "TUI shutdown coordination: Arc<AtomicBool> polled in async block inside tokio::select! arm (100ms interval); cancels CancellationToken when true; after select! exits, store(true) + thread join ensures clean teardown from either side"
  - "Tracing redirect pattern: match on Commands::Serve(args) if !args.no_tui before .init() call — exactly one init path, no double-init risk"
observability_surfaces:
  - "TUI mode: ALL tracing output → .smelt/serve.log (tail -f .smelt/serve.log for live inspection)"
  - "--no-tui mode: tracing → stderr as before"
  - "TUI shutdown logged: tracing::info!(\"TUI requested shutdown\") before cancel"
  - "Ctrl+C logged: tracing::info!(\"Ctrl+C received — cancelling all jobs\") before cancel"
duration: 20min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T04: Wire TUI + tracing redirect into smelt serve; cargo test --workspace green

**Integrated run_tui() into smelt serve with Arc<AtomicBool> shutdown coordination and tracing-appender redirect to .smelt/serve.log in TUI mode — cargo test --workspace: 46 passed, 0 failed.**

## What Happened

Added `tracing-appender = "0.2"` to workspace deps and `smelt-cli` Cargo.toml. Updated `main.rs` to branch the tracing subscriber init: when `smelt serve` runs without `--no-tui`, a file appender writing to `.smelt/serve.log` is used (directory created via `create_dir_all`); all other commands continue using stderr. This prevents tracing output from corrupting the Ratatui alternate screen.

Updated `commands/serve.rs` to spawn the TUI thread via `run_tui()` when `!args.no_tui`, passing `Arc::clone(&state)` and `Arc::clone(&shutdown)`. A fifth arm was added to `tokio::select!` that polls the `shutdown: Arc<AtomicBool>` every 100ms — when the TUI sets it true (user presses `q` or Ctrl+C inside TUI), this arm fires, logs "TUI requested shutdown", and calls `cancel_for_tui.cancel()`. After `select!` exits for any reason, `shutdown.store(true, SeqCst)` is set and the TUI join handle is consumed.

Removed the unused `render` re-export from `serve/mod.rs` and suppressed the intentional `retry_backoff_secs` dead-code warning with an explanatory `#[allow]` attribute.

## Verification

```
cargo build -p smelt-cli       # Finished — no errors, no warnings
cargo test --workspace         # 46 smelt-cli tests + full workspace — 0 failures
smelt serve --help             # shows --config and --no-tui flags
```

All 46 smelt-cli unit tests pass including:
- `test_serve_http_responds_while_running` — HTTP API verified via integration test
- `test_tui_render_no_panic` — TUI render via TestBackend verified
- `test_server_config_roundtrip` / `test_server_config_invalid_max_concurrent` — config tests

## Diagnostics

- TUI mode: `tail -f .smelt/serve.log` for live tracing output
- `GET http://127.0.0.1:8765/api/v1/jobs` for live job state
- TUI shutdown: AtomicBool with SeqCst ordering — inspectable via test assertion or debugger
- If TUI thread panics: `ratatui::restore()` runs (via panic hook from `ratatui::init()`), terminal restored, error printed to stderr via `eprintln!("TUI error: {e}")`

## Deviations

- The `shutdown_poll` clone was renamed from `shutdown` to avoid borrow conflicts inside the `tokio::select!` macro (which holds borrows across arms). This is a standard Rust borrow resolution pattern, not a logic change.
- `render` re-export was dropped from `serve/mod.rs` — it was exported in T03 but never consumed outside `tui.rs` itself.

## Known Issues

None. Manual smoke test with TUI visible was not performed (no interactive terminal in agent session), but all automated must-haves verified.

## Files Created/Modified

- `Cargo.toml` — added `tracing-appender = "0.2"` to `[workspace.dependencies]`
- `crates/smelt-cli/Cargo.toml` — added `tracing-appender.workspace = true` to `[dependencies]`
- `crates/smelt-cli/src/main.rs` — conditional tracing init: file appender for TUI mode, stderr for all other commands
- `crates/smelt-cli/src/commands/serve.rs` — integrated run_tui() with Arc<AtomicBool> shutdown, TUI poll arm in tokio::select!, thread join on exit
- `crates/smelt-cli/src/serve/mod.rs` — removed unused `render` re-export
- `crates/smelt-cli/src/serve/config.rs` — added `#[allow(dead_code)]` to `retry_backoff_secs` field
