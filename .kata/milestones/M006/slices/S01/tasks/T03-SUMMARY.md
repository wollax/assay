---
id: T03
parent: S01
milestone: M006
provides:
  - "Wrapping ↑↓ navigation in handle_event — Down wraps from last to first, Up wraps from first to last"
  - "Enter key transitions App.screen to Screen::MilestoneDetail when milestones is non-empty and selection exists"
  - "Esc key returns App.screen to Screen::Dashboard from any non-Dashboard screen"
  - "Empty-list guard in draw_dashboard — renders placeholder Paragraph and returns early; never calls render_stateful_widget on empty list"
  - "crates/assay-tui/tests/app_state.rs with 7 unit tests covering all state transitions — no terminal required"
  - "aws-lc-rs / rustls-webpki bumped (cargo update) to clear pre-existing RUSTSEC advisories blocking just ready"
key_files:
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/tests/app_state.rs
  - crates/assay-tui/Cargo.toml
  - Cargo.lock
key_decisions:
  - "Down/Up navigation uses guard clause (is_empty check) + wrapping match; empty-list is an explicit no-op, not a panic path"
  - "Esc is now a screen-return action (Dashboard) not a quit; quit is q-only — this matches the plan's wording exactly"
  - "Test fixtures construct Milestone structs directly with Utc::now(); chrono added as dev-dependency to assay-tui/Cargo.toml"
  - "aws-lc-rs updated from 1.16.0→1.16.2 (pulled in aws-lc-sys 0.37.1→0.39.0) to resolve RUSTSEC-2026-0044..0048; rustls-webpki updated to 0.103.10 for RUSTSEC-2026-0049"
patterns_established:
  - "Empty-list guard in draw_dashboard: if milestones.is_empty() { render placeholder; return } placed before ListItem construction — prevents ListState panic"
  - "Integration tests in tests/app_state.rs: construct App directly, call handle_event with synthetic key events, assert App field state — no terminal, no I/O"
observability_surfaces:
  - "App.screen variant — single source of truth for current view; readable in tests and debugger"
  - "tests/app_state.rs — living documentation of every state transition; 7 tests cover all key events"
  - "handle_event bool return — false signals quit; only q triggers this from Dashboard"
duration: 25min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T03: Navigation, empty-state guard, and unit tests

**Wrapping ↑↓/Enter/Esc navigation wired in handle_event; empty-list guard in draw_dashboard; 7 passing unit tests in tests/app_state.rs; just ready green.**

## What Happened

All navigation behavior was updated in `handle_event`:

- **Down**: guard on `is_empty()` then wrapping match — `None | Some(n) if n >= len-1 => 0`, `Some(n) => n+1`. The prior T02 implementation used `.min(len-1)` (clamping), not wrapping.
- **Up**: guard on `is_empty()` then wrapping match — `None | Some(0) => len.saturating_sub(1)`, `Some(n) => n-1`. The prior T02 implementation used `saturating_sub(1)` (clamping).
- **Enter**: added arm — transitions to `Screen::MilestoneDetail` when on Dashboard, milestones non-empty, and selection is Some.
- **Esc**: changed from "quit when on Dashboard/NoProject" to "set screen=Dashboard when NOT on Dashboard/NoProject" — aligned with the plan.

The `draw_dashboard` function received an early-return empty-list guard: `if milestones.is_empty() { render placeholder Paragraph; return }` placed before the `ListItem` construction loop. This prevents `render_stateful_widget` from being called on an empty list (the `ListState` panic pattern documented in S01-RESEARCH.md).

The `MilestoneDetail` stub text in `draw` was updated to "Milestone detail — coming in S03".

A `[dev-dependencies]` section was added to `assay-tui/Cargo.toml` with `chrono.workspace = true` to allow test fixtures to construct `Milestone` structs (which have `DateTime<Utc>` fields) without disk I/O.

During `just ready`, `cargo deny` reported 6 new RUSTSEC advisories in `aws-lc-sys` (0.37.1) and `rustls-webpki` (0.103.9) — all pre-existing in the project (present before T03 changes). `cargo update -p aws-lc-rs` bumped `aws-lc-rs` to 1.16.2 which pulled `aws-lc-sys` to 0.39.0, and `cargo update -p rustls-webpki` bumped it to 0.103.10. All advisories cleared. `cargo fmt --all` was run to fix pre-existing formatting in the `draw_dashboard` function signature.

## Verification

- `cargo test -p assay-tui 2>&1 | grep 'test result'` → `test result: ok. 7 passed; 0 failed` ✓
- `just ready` → exits 0 with "All checks passed" ✓
- `grep -c 'is_empty' crates/assay-tui/src/lib.rs` → 4 (guard present in draw_dashboard + handle_event) ✓
- `cargo build -p assay-tui` → Finished, `target/debug/assay-tui` exists ✓
- `cargo build -p assay-cli` → Finished, `target/debug/assay` exists, `target/debug/assay-tui` still exists (no collision) ✓

## Diagnostics

- `App.screen` — inspect variant to know current view at any point
- `tests/app_state.rs` — 7 tests serve as executable spec for all key→state transitions; run with `cargo test -p assay-tui`
- `handle_event` returns `false` only on `q`; all other keys (including Esc) return `true`

## Deviations

- `cargo update -p aws-lc-rs` and `cargo update -p rustls-webpki` were run to clear pre-existing RUSTSEC advisories. This updated `Cargo.lock` and `aws-lc-sys` transitively. Not in the task plan but required for `just ready` to pass.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/lib.rs` — wrapping Down/Up, Enter→MilestoneDetail, Esc→Dashboard, empty-list guard in draw_dashboard, MilestoneDetail stub text updated
- `crates/assay-tui/tests/app_state.rs` — new file; 7 unit tests for all state transitions
- `crates/assay-tui/Cargo.toml` — added `[dev-dependencies]` with `chrono.workspace = true`
- `Cargo.lock` — updated aws-lc-rs 1.16.0→1.16.2, aws-lc-sys 0.37.1→0.39.0, rustls-webpki 0.103.9→0.103.10
