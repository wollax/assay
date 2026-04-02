---
estimated_steps: 6
estimated_files: 6
---

# T04: Wire TUI + tracing redirect into smelt serve; cargo test --workspace green

**Slice:** S03 — Ratatui TUI + Server Config + Graceful Shutdown
**Milestone:** M006

## Description

Final assembly: integrate the T03 TUI thread into `commands/serve.rs`, add tracing-to-file redirect so stderr doesn't corrupt the TUI display, and ensure `cargo test --workspace` stays green. This closes R023 (parallel dispatch daemon) and R025 (live terminal dashboard) as the final wiring that makes the whole `smelt serve` system operational.

## Steps

1. Add `tracing-appender = "0.2"` to workspace `Cargo.toml` under `[workspace.dependencies]` and to `crates/smelt-cli/Cargo.toml` under `[dependencies]`:
   ```toml
   # workspace Cargo.toml
   tracing-appender = "0.2"
   
   # smelt-cli/Cargo.toml
   tracing-appender.workspace = true
   ```

2. Update `crates/smelt-cli/src/main.rs` to conditionally choose the tracing subscriber before dispatching to `execute()`:
   - Move the tracing subscriber init out of the unconditional block at the top of `main()`
   - When `Commands::Serve(ref args)` with `!args.no_tui`: create `.smelt/` directory if needed (`std::fs::create_dir_all(".smelt").ok()`), then init subscriber with `tracing_appender::rolling::never(".smelt", "serve.log")` as writer
   - For all other commands (and `--no-tui` serve): use the existing stderr path (no change to existing logic)
   - Pattern:
     ```rust
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
     ```

3. Update `crates/smelt-cli/src/commands/serve.rs` to integrate the TUI thread:
   - Add imports: `use std::sync::atomic::{AtomicBool, Ordering}; use crate::serve::run_tui;`
   - Before `tokio::select!`: create `let shutdown = Arc::new(AtomicBool::new(false));`
   - If `!args.no_tui`: spawn TUI thread with `let tui_handle = Some(run_tui(Arc::clone(&state), Arc::clone(&shutdown)));`; else `let tui_handle: Option<std::thread::JoinHandle<()>> = None;`
   - In the `tokio::select!` block, add a fifth arm that polls shutdown:
     ```rust
     _ = async {
         loop {
             if shutdown.load(Ordering::SeqCst) { break; }
             tokio::time::sleep(Duration::from_millis(100)).await;
         }
     } => {
         tracing::info!("TUI requested shutdown");
         cancel_token.cancel();
     }
     ```
   - After the `tokio::select!` block exits (for any reason):
     - Set `shutdown.store(true, Ordering::SeqCst)` to signal the TUI thread to exit
     - If `tui_handle` is `Some(handle)`: `handle.join().ok();`
   - Ensure the Ctrl+C arm also sets `shutdown.store(true, Ordering::SeqCst)` before `cancel_token.cancel()`
   - Add `use std::time::Duration;` if not already present

4. Re-export `run_tui` from `serve/mod.rs` (if T03 didn't already add it):
   - `pub(crate) use tui::run_tui;` in `serve/mod.rs`

5. Fix any dead-code warnings for S02 `pub(crate)` functions (dispatch_loop, run_job_task, DirectoryWatcher, build_router) — now that `commands/serve.rs` uses them, the warnings should disappear. If any `#[allow(dead_code)]` attributes were added in S02, remove them.

6. Run `cargo test --workspace` and fix any failures. Common failure sources:
   - Duplicate `tracing_subscriber::fmt().init()` calls (init must happen exactly once — the conditional in step 2 handles this)
   - MSRV conflicts from `ratatui 0.29` (should be fine with 1.85 floor)
   - `tracing-appender 0.2` version compatibility with `tracing-subscriber 0.3` (both maintained by tokio-rs, API-compatible)
   - Any lingering `#[allow(dead_code)]` that hid a type mismatch

## Must-Haves

- [ ] `cargo test --workspace` → all tests pass, 0 failures
- [ ] `cargo build -p smelt-cli` → no errors, no unused-serve-function dead-code warnings
- [ ] `smelt serve --config examples/server.toml` starts with Ratatui TUI visible in alternate screen showing "smelt serve — jobs" table header
- [ ] `smelt serve --config examples/server.toml --no-tui` starts with tracing output on stderr (no TUI)
- [ ] Pressing `q` in TUI mode triggers clean shutdown (TUI exits, all jobs cancelled, process exits 0)
- [ ] `.smelt/serve.log` is created and contains tracing output when TUI mode is active
- [ ] Shutdown sequence: cancel_token.cancel() → dispatch_loop stops → TUI thread joins → process exits

## Verification

- `cargo test --workspace 2>&1 | tail -5` → "test result: ok. N passed; 0 failed"
- `cargo build -p smelt-cli 2>&1 | grep -E "^error|dead_code"` → no output
- `smelt serve --help` → shows both `smelt serve --config` and `--no-tui` flags with descriptions
- Manual smoke test (if Docker available): `smelt serve --config examples/server.toml` → TUI renders, `curl http://127.0.0.1:8765/api/v1/jobs` returns `[]`, `q` keypress exits cleanly

## Observability Impact

- Signals added/changed: in TUI mode, ALL `tracing::info!/warn!/error!` output goes to `.smelt/serve.log` — zero corruption of TUI display; in `--no-tui` mode, tracing goes to stderr as before; TUI shutdown path logs "TUI requested shutdown" or "Ctrl+C received — cancelling all jobs" before cancelling
- How a future agent inspects this: `tail -f .smelt/serve.log` during TUI mode; `GET /api/v1/jobs` for live job state; exit code 0 confirms clean shutdown
- Failure state exposed: if TUI thread panics, `ratatui::restore()` runs (via panic hook from `ratatui::init()`), terminal is restored, error printed to stderr; `shutdown.store(true)` ensures tokio runtime also exits

## Inputs

- `crates/smelt-cli/src/serve/tui.rs` — `run_tui(state, shutdown)` from T03
- `crates/smelt-cli/src/commands/serve.rs` — `execute()` function from T02
- `crates/smelt-cli/src/main.rs` — tracing init block to conditionally branch
- `crates/smelt-cli/Cargo.toml` — needs `tracing-appender.workspace = true` added
- `Cargo.toml` — needs `tracing-appender = "0.2"` in `[workspace.dependencies]`

## Expected Output

- `crates/smelt-cli/src/main.rs` — conditional tracing init: file appender for TUI mode, stderr for all other modes
- `crates/smelt-cli/src/commands/serve.rs` — `shutdown: Arc<AtomicBool>` coordinating TUI thread + tokio runtime; TUI thread spawned when `!args.no_tui`; thread joined on shutdown
- `Cargo.toml` — `tracing-appender = "0.2"` in workspace deps
- `crates/smelt-cli/Cargo.toml` — `tracing-appender.workspace = true` in dependencies
- Clean `cargo test --workspace` output — 0 failures, all existing tests still passing
