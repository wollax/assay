# S01: App Scaffold, Dashboard, and Binary Fix

**Goal:** Fix the missing `[[bin]]` declaration so `assay-tui` produces a real binary, then replace the 42-line stub with a full `App`/`Screen` Ratatui application that loads live milestone data from `assay-core` and renders a dashboard — list of milestones with status badges, chunk progress fractions, and gate pass/fail counts — with arrow-key navigation and `q` to quit.
**Demo:** `cargo build -p assay-tui` produces `target/debug/assay-tui`; launching it on a project with `.assay/` shows a list of milestones with name, status badge, chunk fraction (e.g. `2/4`), and gate pass/fail column loaded from real files; `↑↓` moves the selection; `q` quits cleanly; launching on a directory without `.assay/` shows a clean "Not an Assay project" message and exits without panic.

## Must-Haves

- `cargo build -p assay-tui` produces `target/debug/assay-tui`; `cargo build -p assay-cli` still produces `target/debug/assay` (no collision)
- `App::new(project_root)` with no `.assay/` directory sets `screen: Screen::NoProject` — verified by unit test
- `App::new(project_root)` with `.assay/` but zero milestones renders empty-state text — verified by unit test
- `handle_event(Up)` and `handle_event(Down)` update `list_state` selection — verified by unit tests
- `handle_event(Char('q'))` returns `true` (quit signal) — verified by unit test
- Dashboard shows milestone name, `MilestoneStatus` badge, chunk progress fraction, and gate pass/fail column from real `history::list` + `history::load` data — verified by `#[cfg(test)]` test using `tempfile::TempDir` + public `assay_core::milestone::milestone_save` + `assay_core::history::save_run` to build fixture state
- No panic when `.assay/` is absent or when milestone list is empty — verified by unit tests
- `just ready` passes after S01 (`cargo fmt`, `cargo clippy`, `cargo test --workspace`, `cargo deny`)

## Proof Level

- This slice proves: integration (real `milestone_scan` + `history` data; event-loop state transitions)
- Real runtime required: yes — visual inspection via `cargo run -p assay-tui` on a project with fixtures
- Human/UAT required: no — unit + integration tests cover all must-haves; visual verify is optional confirmation

## Verification

- `cargo test -p assay-tui` — all unit and integration tests in `crates/assay-tui/src/app.rs` pass
- `cargo build -p assay-tui && ls -la target/debug/assay-tui` — binary present
- `cargo build -p assay-cli && ls -la target/debug/assay` — CLI binary present, no collision
- `just ready` — full workspace check passes
- Manual: `cargo run -p assay-tui` in a project with `.assay/milestones/` shows live dashboard; `q` quits; `↑↓` navigates

## Observability / Diagnostics

- Runtime signals: Screen enum variant is the state machine; no additional logging needed for S01
- Inspection surfaces: `App.screen`, `App.milestones`, `App.list_state` — inspectable in test context; `Screen::NoProject` renders explicit diagnostic text to terminal
- Failure visibility: `draw_no_project()` renders a clear error message visible to the user; loading errors (bad config, malformed TOML) are silently degraded (config: None, milestones: vec![]) rather than panicking — the user sees an empty dashboard, not a crash
- Redaction constraints: none — no secrets in milestone/config display

## Integration Closure

- Upstream surfaces consumed: `assay_core::milestone::milestone_scan(assay_dir)`, `assay_core::milestone::cycle_status(assay_dir)`, `assay_core::config::load(project_root)`, `assay_core::history::list(assay_dir, spec_name)`, `assay_core::history::load(assay_dir, spec_name, run_id)`, `assay_types::{Milestone, MilestoneStatus, ChunkRef}`
- New wiring introduced in this slice: `[[bin]]` declaration; `App` + `Screen` state machine; `run()` polling event loop; `draw()` dispatch; dashboard render with live data
- What remains before the milestone is truly usable end-to-end: S02 (wizard), S03 (chunk detail), S04 (settings), S05 (help overlay + polish)

## Tasks

- [x] **T01: Binary fix, `App`/`Screen` scaffold, and failing tests** `est:30m`
  - Why: The `[[bin]]` declaration is the blocking prerequisite — without it `cargo build -p assay-tui` produces nothing. The scaffold defines the `App` and `Screen` types and writes the test assertions that will drive T02 implementation.
  - Files: `crates/assay-tui/Cargo.toml`, `crates/assay-tui/src/main.rs`, `crates/assay-tui/src/app.rs`
  - Do: (1) Add `[[bin]] name = "assay-tui" path = "src/main.rs"` to `Cargo.toml`. (2) Create `src/app.rs` with `App { screen, milestones, list_state, project_root, config }`, `Screen` enum (`Dashboard`, `NoProject`), stub `App::new()`, stub `run()`, stub `handle_event()`, stub `draw()`. (3) Update `main.rs` to call `app::run(terminal)` instead of the inline loop. (4) Add `#[cfg(test)]` module in `app.rs` with unit tests covering: `no_assay_dir → Screen::NoProject`, `handle_event(Up/Down)` list state changes, `handle_event('q') → true`, empty milestone list guard. (5) `cargo build -p assay-tui` — verify binary produced. Tests will fail at this stage (stubs return wrong values) — that is expected and correct.
  - Verify: `cargo build -p assay-tui && ls target/debug/assay-tui` exits 0; `cargo build -p assay-cli && ls target/debug/assay` exits 0; `cargo test -p assay-tui` compiles but some tests fail (stubs not yet implemented)
  - Done when: Binary produced; tests compile and run (some failing is fine); no binary name collision

- [ ] **T02: Dashboard rendering, event loop, and gate data — all tests pass** `est:1.5h`
  - Why: Implements all the real behavior that makes the tests from T01 pass and delivers the slice demo: live milestone data, gate pass/fail column, polling event loop, arrow-key navigation, no-project guard.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/main.rs`
  - Do: (1) Implement `App::new(project_root)`: check `.assay/` dir existence; if absent set `screen: Screen::NoProject`; otherwise call `milestone_scan(assay_dir)` (handle `Err` → `vec![]`), call `config::load(project_root)` (handle `Err` → `None`), set `list_state` with `ListState::default().with_selected(Some(0))` if milestones non-empty else `None`. (2) Add `GateSummary { passed: u32, failed: u32 }` and compute it in `App::new()` via `history::list(assay_dir, chunk_slug)` → last entry → `history::load` → `record.summary.passed/failed`; store as `Vec<(String, GateSummary)>` keyed by milestone slug. (3) Implement `run(terminal)` with polling event loop: `event::poll(Duration::from_millis(250))`; if event → `event::read()` → `handle_event()`; `terminal.draw(|f| draw(f, &mut app))`; loop exits when `handle_event` returns `true`. (4) Implement `handle_event(event, app) -> bool`: `KeyCode::Char('q') → true`; `KeyCode::Up → app.list_state.select_previous()`; `KeyCode::Down → app.list_state.select_next()`; `Event::Resize → false`; default `false`. Guard `Up/Down` to no-op when `milestones.is_empty()`. (5) Implement `draw(frame, app)`: match on `app.screen` → call `draw_dashboard` or `draw_no_project`. (6) Implement `draw_dashboard(frame, milestones, list_state, gate_data)`: render `List` widget with `render_stateful_widget`; each item shows `"{name}  [{status}]  {completed}/{total}  ✓{passed} ✗{failed}"`; use `Layout::vertical([Length(1), Fill(1), Length(1)])` for title bar / body / hint bar. (7) Implement `draw_no_project(frame)`: render centered `Paragraph` with "Not an Assay project — run `assay init` first". (8) Implement `draw_no_milestones(frame)`: render centered `Paragraph` with "No milestones — run `assay plan`". (9) Run unit tests; fix until all pass. (10) `just ready`.
  - Verify: `cargo test -p assay-tui` — all tests pass; `just ready` passes; `cargo run -p assay-tui` launches on a project with `.assay/` and shows milestones; `q` quits; `↑↓` moves selection; no panic on empty `.assay/`
  - Done when: All unit tests in `app.rs` pass; `just ready` green; binary produces correct output on fixture data

## Files Likely Touched

- `crates/assay-tui/Cargo.toml`
- `crates/assay-tui/src/main.rs`
- `crates/assay-tui/src/app.rs` (new)
