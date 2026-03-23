---
id: T04
parent: S01
milestone: M007
provides:
  - assay-harness dependency in assay-tui
  - App.event_tx field (Option<mpsc::Sender<TuiEvent>>) wired from run()
  - r key handler in Dashboard arm spawning agent via launch_agent_streaming
  - TuiEvent moved to shared assay_tui::event module (event.rs)
  - Two-channel exit-code design for AgentDone (no JoinHandle on App)
  - All 3 agent_run integration tests green; just ready passes
key_files:
  - crates/assay-tui/Cargo.toml
  - crates/assay-tui/src/event.rs
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/main.rs
key_decisions:
  - TuiEvent moved from main.rs to new src/event.rs module so app.rs can reference it without circular imports
  - Two-channel design implemented as specified — line_tx/line_rx for streamed stdout, exit_tx/exit_rx for exit code; bridge thread forwards both into TuiEvent channel; App.agent_thread is None after r key press
  - ManifestSession constructed field-by-field (no Default derive on the type)
  - tempfile dir leaked via std::mem::forget to keep harness config alive for subprocess duration
  - handle_agent_done simplified to defensive-only join (always None in production path)
patterns_established:
  - Two-channel streaming pattern — separate mpsc channels for lines and exit code, bridged into single TuiEvent channel; eliminates JoinHandle ownership conflicts
  - Shared event module pattern — TuiEvent in src/event.rs, imported by both main.rs and app.rs
observability_surfaces:
  - App.event_tx presence indicates agent can be spawned from r key (None means no-op)
  - Screen::AgentRun.lines captures full agent stdout verbatim
  - Screen::AgentRun.status is AgentRunStatus::Done{exit_code} or Failed{exit_code} after AgentDone
  - Non-zero exit_code renders red "Failed (exit N)" in TUI status bar
duration: 1 session
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T04: Wire `r` key handler, add `assay-harness` dependency, complete integration tests

**Added `assay-harness` to `assay-tui`, moved `TuiEvent` to a shared module, added `App.event_tx`, and implemented the `r` key handler using a two-channel design — all 3 agent_run integration tests pass and `just ready` is green.**

## What Happened

Step 1: Added `assay-harness.workspace = true` and `tempfile.workspace = true` to `[dependencies]` in `crates/assay-tui/Cargo.toml`.

Step 2: `TuiEvent` was defined in `main.rs` but needed by `app.rs`. Moved it to a new `crates/assay-tui/src/event.rs` module and added `pub mod event` to `lib.rs`. Updated `main.rs` to import from `assay_tui::event::TuiEvent`.

Step 3: Added `use std::sync::mpsc` and `use crate::event::TuiEvent` to `app.rs`. Added `event_tx: Option<mpsc::Sender<TuiEvent>>` field to `App`, initialized to `None` in `with_project_root`. In `main.rs`, set `app.event_tx = Some(tx.clone())` after channel creation.

Step 4: Implemented `handle_r_key()` as a private method on `App`. Uses the two-channel design: `(line_tx, line_rx)` for streamed stdout lines, `(exit_tx, exit_rx)` for the exit code. The JoinHandle from `launch_agent_streaming` is moved into a dedicated exit-code thread that joins it and sends the code via `exit_tx`. A bridge thread drains `line_rx` forwarding `TuiEvent::AgentLine`, then receives from `exit_rx` and sends `TuiEvent::AgentDone { exit_code }`. `App.agent_thread` is set to `None`. The temp dir is leaked via `std::mem::forget`.

`ManifestSession` does not derive `Default`, so it was constructed field-by-field.

Step 5: Updated `handle_agent_done` to use `if let ... && let Err(e) = ...` (collapsible-if fix for clippy). The join is now defensive-only; in production the handle is always `None`.

Step 6: Ran `cargo fmt --all` to fix formatting. `just ready` passed (fmt + lint + test + deny all green).

## Verification

```
cargo test -p assay-tui --test agent_run
# 3/3 pass: agent_run_streams_lines_and_transitions_to_done,
#            agent_run_failed_exit_code_shows_failed_status,
#            agent_run_r_key_on_no_project_is_noop

cargo test -p assay-tui
# all tests pass (wizard + agent_run)

cargo test -p assay-core --test pipeline_streaming
# 3/3 pass

just ready
# fmt + lint + test + deny all pass
```

## Diagnostics

- `match &app.screen { Screen::AgentRun { lines, status, .. } => ... }` — inspect line buffer and status after driving events
- `app.event_tx.is_some()` — true when inside a real `run()` loop, false in unit tests (r key no-ops without it)
- Bridge thread logs nothing — silent disconnect on receiver drop is intentional
- `handle_agent_done` logs join errors via `eprintln!` (non-fatal, defensive path only)

## Deviations

- `TuiEvent` moved to `src/event.rs` (new shared module). The plan described `TuiEvent` as living in `main.rs` and implied passing the sender through `App`. Moving to a shared module was necessary to avoid circular imports and is the correct Rust architecture.
- `ManifestSession::default()` was not available — constructed field-by-field instead. No behavioral difference.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added `assay-harness.workspace = true`, moved `tempfile` to runtime deps
- `crates/assay-tui/src/event.rs` — new file; `TuiEvent` enum (Key, Resize, AgentLine, AgentDone)
- `crates/assay-tui/src/lib.rs` — added `pub mod event`
- `crates/assay-tui/src/app.rs` — added `use std::sync::mpsc`, `use crate::event::TuiEvent`; `App.event_tx` field; `handle_r_key()` method; updated `handle_agent_done`
- `crates/assay-tui/src/main.rs` — removed `TuiEvent` definition; added `use assay_tui::event::TuiEvent`; `app.event_tx = Some(tx.clone())`
