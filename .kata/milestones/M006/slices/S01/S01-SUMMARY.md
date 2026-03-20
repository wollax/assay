---
id: S01
parent: M006
milestone: M006
provides:
  - "[[bin]] declaration in assay-tui/Cargo.toml → target/debug/assay-tui binary (no collision with assay-cli's target/debug/assay)"
  - "App struct: screen, milestones, gate_data, list_state, project_root, config fields"
  - "Screen enum: Dashboard and NoProject variants (Wizard/Settings/MilestoneDetail/ChunkDetail slots reserved for S02–S04)"
  - "GateSummary struct: passed/failed u32 fields per milestone"
  - "App::new(project_root) — checks .assay/, loads config + milestones, populates gate_data from history"
  - "compute_gate_data(assay_dir, milestones) — parallel-indexed Vec<GateSummary> from history::list + history::load"
  - "run(terminal) — 250ms polling event loop (event::poll not blocking event::read)"
  - "handle_event(event, app) -> bool — q/Q/Esc quit; Up/Down navigate with wrap; Resize noop"
  - "draw(frame, app) — dispatches to draw_dashboard / draw_no_project / draw_no_milestones"
  - "draw_dashboard() — 3-zone layout (header/body/footer); milestone list with status badge, chunk fraction, gate counts via render_stateful_widget"
  - "draw_no_project() — 'Not an Assay project — run `assay init` first'"
  - "draw_no_milestones() — 'No milestones — run `assay plan`'"
  - "status_badge() — MilestoneStatus → [Draft]/[InProgress]/[Verify]/[Complete]"
  - "5 unit tests all passing: no_assay_dir → NoProject, q → true, Up/Down on empty (no panic), Down moves selection, gate_data loaded from history fixtures"
requires: []
affects:
  - S02
  - S03
  - S04
  - S05
key_files:
  - crates/assay-tui/Cargo.toml
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/src/app.rs
key_decisions:
  - "D088 — assay-tui binary name is assay-tui; assay-cli keeps assay; no collision"
  - "D089 — App struct + Screen enum as central state machine; free functions for render (no Widget trait impls)"
  - "D091 — Data loading is synchronous on construction (App::new) and on navigation; no background threads in S01"
  - "Silent degradation: config and milestone load errors produce empty data (Dashboard); only absent .assay/ produces Screen::NoProject"
  - "compute_gate_data is parallel-indexed with milestones — both use milestone slug sort so order is stable"
  - "list_state.select_previous()/select_next() used instead of manual index arithmetic (built-in ratatui API with wrap)"
patterns_established:
  - "App::new(project_root) is the canonical constructor — .assay/ presence check first, then data load"
  - "compute_gate_data(assay_dir, milestones) is the canonical gate history loader — takes milestone slice, returns parallel Vec"
  - "All TUI data load errors are silently degraded — TUI never panics on bad data"
  - "draw_dashboard uses render_stateful_widget (not render_widget) for selection state"
  - "app.screen variant is the sole TUI state machine signal — downstream slices add new Screen variants"
observability_surfaces:
  - "app.screen — readable in tests; Screen::NoProject means .assay/ absent"
  - "draw_no_project() renders actionable message to terminal — user sees what to do"
  - "draw_no_milestones() renders actionable message — user sees what to do"
  - "app.milestones.len() == app.gate_data.len() invariant — maintained by compute_gate_data; violation would cause zip panic"
drill_down_paths:
  - .kata/milestones/M006/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M006/slices/S01/tasks/T02-SUMMARY.md
duration: 45min
verification_result: passed
completed_at: 2026-03-20T14:15:00Z
---

# S01: App Scaffold, Dashboard, and Binary Fix

**`assay-tui` is a real binary with a live Ratatui dashboard loading milestone data, gate history, and status badges from `assay-core` — 5 unit tests green, both binaries collision-free.**

## What Happened

S01 resolved the two blocking issues in `assay-tui` and delivered the full dashboard foundation.

**T01 — Binary fix and scaffold:** The `assay-tui` crate had no `[[bin]]` declaration, so `cargo build -p assay-tui` produced nothing. Adding `[[bin]] name = "assay-tui" path = "src/main.rs"` resolved it. `App`, `Screen`, and `GateSummary` types were created in a new `src/app.rs`. Rather than writing stubs, the implementation was complete enough to pass all 4 unit tests immediately: `App::new()` checks `.assay/` existence and sets `Screen::NoProject` if absent; `handle_event()` routes `q` to quit and arrow keys to wrap-around list navigation. `main.rs` became a 15-line delegator (panic hook → `ratatui::init()` → `app::run(terminal)` → `ratatui::restore()`).

**T02 — Dashboard rendering and gate data:** `compute_gate_data()` was added, iterating each milestone's chunks, calling `history::list()` + `history::load()` for the latest run, and accumulating `passed`/`failed` counts per milestone. The polling event loop was upgraded from blocking `event::read()` to `event::poll(250ms)`, keeping headroom for future background refresh. `draw_dashboard()` uses a 3-zone `Layout::vertical([Length(1), Fill(1), Length(1)])` with a bold header, selection-aware milestone list (formatted as `name | [Status] | done/total | ✓passed ✗failed`), and a footer keybinding hint. A fifth integration test (`test_gate_data_loaded_from_history`) validates the full history loading path using `milestone_save` + `history::save_run` fixtures in a `TempDir`.

## Verification

- `cargo test -p assay-tui` — 5 passed, 0 failed
- `cargo build -p assay-tui && ls -la target/debug/assay-tui` — binary present (11.9 MB)
- `cargo build -p assay-cli && ls -la target/debug/assay` — CLI binary present (40.8 MB), no collision
- `cargo fmt --all -- --check` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo test --workspace` — 769+ tests, all pass
- `cargo deny check` — fails on pre-existing `aws-lc-sys` RUSTSEC-2026-0044..0048 advisories (transitive via `jsonschema` dev-dep of `assay-types`; pre-dates S01; not introduced here)

## Requirements Advanced

- R049 (TUI project dashboard) — Dashboard rendering with real `milestone_scan` + `history` data is now live; milestone name, `MilestoneStatus` badge, chunk progress fraction, and gate pass/fail column all rendered from real files; keyboard navigation and `q`-to-quit working

## Requirements Validated

- R049 — All S01 must-haves are met: binary produced (no collision), `NoProject` screen unit-tested, Up/Down/quit unit-tested, gate data integration-tested, `just ready` fails only on pre-existing `cargo deny` advisory unrelated to this slice

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

T01 was planned as "write stubs that fail, then T02 makes them pass." Instead, the real implementation was achievable within T01's scope, so all 4 tests passed at T01 and T02 focused on gate history loading + richer rendering rather than making stubs pass. This is a positive deviation — the slice is stronger.

`draw_no_project()` message was updated in T02 from "Not an Assay project — no .assay/ directory found. Press q to quit." to "Not an Assay project — run `assay init` first" — actionable instruction preferred over diagnostic path detail.

## Known Limitations

- `cargo deny check` fails on pre-existing `aws-lc-sys` advisories (RUSTSEC-2026-0044..0048) pulled by `reqwest → rustls → aws-lc-rs` in `jsonschema`'s dep chain. Fix requires upgrading `aws-lc-sys` ≥ 0.39.0 workspace-wide. Unrelated to S01.
- Dashboard row format is a single wide string — no column alignment via `Table` widget. Acceptable for S01; S05 (polish) can upgrade if needed.
- Gate history loads at `App::new()` only — no live refresh while TUI is running. Sufficient for S01; S07 adds background refresh for agent spawning.
- `Screen` enum currently has only `Dashboard` and `NoProject` — other variants (`Wizard`, `Settings`, `MilestoneDetail`, `ChunkDetail`) are declared as `allow(dead_code)` stubs awaiting S02–S04.

## Follow-ups

- S02: `Screen::Wizard(WizardState)` + multi-step form; `App.project_root` already available
- S03: `Screen::MilestoneDetail` + `Screen::ChunkDetail`; `Enter` key in dashboard is currently a no-op — needs wiring to `Screen::MilestoneDetail`
- S04: `Screen::Settings`; `App.config` already loaded and available
- S05: Replace single-string dashboard row format with `Table` widget for column alignment if visual alignment is needed; address `cargo deny` advisory

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added `[[bin]]` section, `assay-types` dep, `tempfile` + `chrono` dev-deps
- `crates/assay-tui/src/main.rs` — stripped to 15-line delegator; panic hook + ratatui init/restore preserved
- `crates/assay-tui/src/app.rs` — new file: App/Screen/GateSummary types; compute_gate_data; run/handle_event/draw; 5 unit tests

## Forward Intelligence

### What the next slice should know

- `app.screen` is the state machine root — add new `Screen` variants and match arms in `draw()` and `handle_event()`; the compiler will catch missing arms
- `App.project_root: Option<PathBuf>` — it's `None` only in the `Screen::NoProject` case (when `.assay/` is absent); all S02+ screens should check `project_root.as_ref()` before doing I/O
- `Enter` key is currently unhandled in `handle_event()` — S02/S03 need to add `KeyCode::Enter` dispatch based on `app.screen` variant
- `compute_gate_data` is parallel-indexed with `milestones` — any code that adds or removes milestones from `App` must ensure `gate_data` is recomputed in the same pass

### What's fragile

- `milestones.len() == gate_data.len()` invariant — if these diverge (e.g. a future code path appends a milestone without recomputing gate data), `draw_dashboard`'s `zip` will silently truncate. Add an assertion if this invariant is ever at risk.
- `list_state` becomes stale if `milestones` is repopulated — reset `list_state` to `ListState::default().with_selected(Some(0))` any time the milestone list is refreshed

### Authoritative diagnostics

- `app.screen` variant — check first; `Screen::NoProject` means `.assay/` absent; `Screen::Dashboard` with empty `milestones` means scan returned zero results
- `cargo test -p assay-tui` — 5 tests; if any fail, the event-loop or data-loading invariants are broken
- `ls target/debug/assay-tui` vs `ls target/debug/assay` — binary collision check is the first thing to verify after any Cargo.toml change

### What assumptions changed

- Plan assumed T01 would produce failing stubs and T02 would make them pass — in practice, the full implementation fit in T01, so both tasks produced passing code. The overall slice is more complete, not less.
- `cargo deny check` was expected to pass in `just ready` — it fails on pre-existing advisories not introduced by S01. The `just ready` check for S01 is effectively "fmt + clippy + test all pass," which they do.
