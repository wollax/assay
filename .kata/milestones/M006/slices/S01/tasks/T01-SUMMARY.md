---
id: T01
parent: S01
milestone: M006
provides:
  - "[[bin]] section in assay-tui/Cargo.toml produces target/debug/assay-tui binary"
  - "Screen enum (Dashboard, NoProject) — TUI state machine root"
  - "GateSummary struct (passed/failed u32 fields)"
  - "App struct with milestones, gate_data, list_state, project_root, config fields"
  - "app::run(), handle_event(), draw() — full working event loop (not stubs)"
  - "main.rs delegates to app::run() with preserved panic hook + ratatui init/restore"
  - "4 passing unit tests covering NoProject screen, q-to-quit, Up/Down navigation"
key_files:
  - crates/assay-tui/Cargo.toml
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/src/app.rs
key_decisions:
  - "Implemented real App::new() logic (checks .assay/, loads config/milestones) rather than pure stubs — all 4 tests pass immediately rather than requiring T02 to complete them"
  - "Silent degradation: config and milestone load failures produce empty data (screen=Dashboard), not panics; only missing .assay/ triggers Screen::NoProject"
  - "Wrap-around navigation: Down from last item goes to 0, Up from 0 goes to last"
patterns_established:
  - "app.screen variant is the sole TUI state machine signal; Screen::NoProject renders explicit diagnostic text"
  - "App::new(project_root) is the canonical constructor — checks .assay/ presence first"
observability_surfaces:
  - "app.screen — inspectable in tests; Screen::NoProject means no .assay/ dir found"
  - "draw_no_project() renders 'Not an Assay project' message to terminal — visible to user"
duration: 20min
verification_result: passed
completed_at: 2026-03-20T13:00:00Z
blocker_discovered: false
---

# T01: Binary fix, `App`/`Screen` scaffold, and failing tests

**`assay-tui` now produces a real binary with a full App/Screen event loop loading live milestone data; all 4 unit tests pass.**

## What Happened

The `assay-tui` crate was missing its `[[bin]]` declaration (D088). Added it along with `assay-types` and `chrono` (dev) as dependencies. Created `src/app.rs` with the full `App`, `Screen`, and `GateSummary` types. Rather than writing pure stubs that would fail, the implementation was complete enough that all tests pass at T01:

- `App::new()` checks for `.assay/` and sets `Screen::NoProject` if absent, otherwise loads config and milestones via `assay_core::config::load()` and `assay_core::milestone::milestone_scan()`.
- `handle_event()` handles `q` (quit), `Up` (wrap-around decrement), `Down` (wrap-around increment) correctly.
- `draw()` dispatches to `draw_no_project()` or `draw_dashboard()` based on `app.screen`.

`main.rs` was stripped to a 15-line delegator: panic hook → `ratatui::init()` → `app::run(terminal)` → `ratatui::restore()`.

## Verification

```
cargo build -p assay-tui && ls -la target/debug/assay-tui   → binary present (5.4 MB)
cargo build -p assay-cli && ls -la target/debug/assay        → cli binary present (40.8 MB, no collision)
cargo test -p assay-tui                                      → 4 passed, 0 failed
cargo check --workspace                                      → Finished (warnings only, pre-existing)
```

## Diagnostics

- `app.screen` variant is the primary state signal; `Screen::NoProject` triggers the diagnostic render path.
- `draw_no_project()` renders "Not an Assay project — no .assay/ directory found. Press q to quit." directly to the terminal.
- Config/milestone load failures silently degrade — empty dashboard, not a crash.

## Deviations

The task plan expected tests to fail (stubs) and T02 to make them pass. Instead, the full implementation was achievable within this task, so all 4 tests pass at T01. T02 will focus on richer rendering (status badges, chunk fractions, gate columns) and integration testing.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added `[[bin]]` section, `assay-types` dep, `tempfile` + `chrono` dev-deps
- `crates/assay-tui/src/main.rs` — stripped to 15-line delegator; preserved panic hook + ratatui init/restore
- `crates/assay-tui/src/app.rs` — new file: App, Screen, GateSummary types; run/handle_event/draw; 4 unit tests
