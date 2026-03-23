---
id: T03
parent: S03
milestone: M006
provides:
  - run_tui() spawns std::thread background TUI thread using ratatui::init()/restore() with panic hook
  - tui_loop() polls crossterm events every 250ms; exits on 'q', Ctrl+C, or shutdown AtomicBool
  - render() snapshots Arc<Mutex<ServerState>> jobs then releases lock before rendering job table
  - test_tui_render_no_panic verifies empty-state and one-job-state renders via TestBackend
  - ratatui = "0.29" and crossterm = "0.28" added to workspace and smelt-cli Cargo.toml
key_files:
  - crates/smelt-cli/src/serve/tui.rs
  - crates/smelt-cli/src/serve/mod.rs
  - Cargo.toml
  - crates/smelt-cli/Cargo.toml
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - Used std::thread::spawn (not tokio::spawn) for TUI to keep crossterm blocking I/O off the async runtime
  - Arc<AtomicBool> shutdown flag is bidirectional: tokio side sets it to stop TUI, TUI side sets it to stop tokio
  - render() clones job snapshot inside a short lock scope, then drops lock before ratatui renders — avoids holding Mutex across I/O
patterns_established:
  - TUI shutdown coordination pattern: Arc<AtomicBool> checked at loop top + set on exit ensures clean teardown from either side
  - ratatui panic hook installed by ratatui::init(); always paired with ratatui::restore() in the JoinHandle closure
observability_surfaces:
  - eprintln!("TUI error: {e}") after ratatui::restore() — TUI errors surface to stderr without leaving terminal in raw mode
  - test_tui_render_no_panic via TestBackend — CI-safe render verification; TestBackend::buffer() can inspect cell content in future tests
duration: 10min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T03: Ratatui TUI background thread

**Ratatui TUI thread with Arc<AtomicBool> shutdown coordination, crossterm key handling, and TestBackend-verified job table render**

## What Happened

Added `ratatui = "0.29"` and `crossterm = "0.28"` to workspace and smelt-cli Cargo.toml. Created `crates/smelt-cli/src/serve/tui.rs` with three public functions:

- `run_tui()` — spawns a `std::thread` (not tokio task) that calls `ratatui::init()`, runs `tui_loop()`, then always calls `ratatui::restore()` before setting the shutdown flag.
- `tui_loop()` — polls crossterm events every 250ms, exits on `q`/Ctrl+C or when the `AtomicBool` shutdown flag is set.
- `render()` — acquires the `Arc<Mutex<ServerState>>` lock briefly to clone job data, releases it, then renders a 5-column job table (ID, Manifest, Status, Attempt, Elapsed) via ratatui widgets.

Registered `pub(crate) mod tui;` and re-exports in `serve/mod.rs`. Added `test_tui_render_no_panic` to `serve/tests.rs` using `TestBackend` — safe to run in CI with no real terminal.

## Verification

- `cargo test -p smelt-cli serve::tests::test_tui_render_no_panic -- --nocapture` → 1 test passed
- `cargo build -p smelt-cli` → Finished with no errors (warnings only for unused symbols, expected until T04 wires run_tui into serve entrypoint)
- `grep "ratatui" Cargo.toml` → `ratatui = "0.29"` present in workspace deps

## Diagnostics

- TUI errors: `eprintln!("TUI error: {e}")` after `ratatui::restore()` — stderr visible in `--no-tui` mode and in `.smelt/serve.log` when TUI active
- Render panics: `test_tui_render_no_panic` via `TestBackend` catches any panic in the render path; `TestBackend::buffer()` can inspect exact cell content in future tests
- Shutdown signal: `AtomicBool` with `SeqCst` ordering — inspectable via debugger or test assertion if TUI doesn't exit

## Deviations

None — implementation matches the task plan exactly.

## Known Issues

None — `run_tui` and `render` show dead_code warnings until T04 wires them into the serve entrypoint. This is expected and will resolve in T04.

## Files Created/Modified

- `crates/smelt-cli/src/serve/tui.rs` — new file: run_tui(), tui_loop(), render()
- `crates/smelt-cli/src/serve/mod.rs` — added pub(crate) mod tui; and re-exports
- `Cargo.toml` — ratatui = "0.29" and crossterm = "0.28" added to [workspace.dependencies]
- `crates/smelt-cli/Cargo.toml` — ratatui.workspace = true and crossterm.workspace = true added to [dependencies]
- `crates/smelt-cli/src/serve/tests.rs` — test_tui_render_no_panic added
