# S01: App Scaffold, Dashboard, and Binary Fix

**Goal:** Replace the 42-line `assay-tui` stub with a real Ratatui application: `App` + `Screen` type hierarchy, a working dashboard that loads live milestone data from `assay-core`, keyboard navigation, no-project guard, and an explicit `[[bin]]` declaration in `Cargo.toml`.
**Demo:** `cargo build -p assay-tui` produces `target/debug/assay-tui`; launching it on any project with `.assay/milestones/` shows a live dashboard with milestones (name, status badge, chunk progress fraction) loaded from `milestone_scan`; ↑↓ navigates the list; `q` quits; launching on a directory with no `.assay/` shows a clean "Not an Assay project" message and exits without panic.

## Must-Haves

- `[[bin]] name = "assay-tui" path = "src/main.rs"` present in `crates/assay-tui/Cargo.toml`
- `cargo build -p assay-tui` produces `target/debug/assay-tui`; `cargo build -p assay-cli` produces `target/debug/assay`; no binary name collision
- `App` struct holds `screen: Screen`, `milestones: Vec<Milestone>`, `list_state: ListState`, `project_root: Option<PathBuf>`, `config: Option<Config>`, `show_help: bool`
- `Screen` enum has all variants: `Dashboard`, `MilestoneDetail`, `ChunkDetail`, `Wizard(WizardState)`, `Settings`, `NoProject`; `WizardState` is a stub struct present in the codebase
- Run loop split into `draw(frame, &App)` (free fn) and `handle_event(&mut App, Event) -> bool` (free fn); no logic inside `terminal.draw()` closure
- Dashboard renders a `List` of milestones with name, status badge string, and chunk progress fraction (e.g. `2/4`) loaded from `milestone_scan`; data is real, not hardcoded
- `config::load` result stored in `App.config`; load is guarded (only called when `.assay/` exists)
- ↑↓ arrow keys move `ListState` selection; wrapping at both ends
- `q` quits from any screen; `Esc` returns to Dashboard from any other screen
- `Enter` on a dashboard item transitions to `Screen::MilestoneDetail` stub (renders placeholder text)
- No `.assay/` → `Screen::NoProject` rendered with clear message; no panic; exits cleanly on `q`/`Esc`
- Empty milestones list → empty-state placeholder rendered instead of panicking `ListState`
- Manual panic hook removed; `ratatui::init()` internal hook is relied on (per research)
- `crates/assay-tui/tests/app_state.rs` exists with tests covering: `navigate_up_down`, `quit_returns_false`, `enter_transitions_to_milestone_detail`, `esc_returns_to_dashboard`, `no_project_screen_set_when_assay_dir_absent`, `empty_milestones_does_not_panic_list_state`
- `cargo test -p assay-tui` passes; `just ready` passes

## Proof Level

- This slice proves: integration
- Real runtime required: yes (manual launch against a project with `.assay/` to visually confirm dashboard data)
- Human/UAT required: yes (visual check; keyboard navigation; no-project guard)

## Verification

- `cargo build -p assay-tui 2>&1 | grep -c 'Finished'` → 1; confirm `target/debug/assay-tui` exists
- `cargo build -p assay-cli 2>&1 | grep -c 'Finished'` → 1; confirm `target/debug/assay` exists and `target/debug/assay-tui` still exists (no collision)
- `cargo test -p assay-tui 2>&1 | grep -E 'test result'` → all 6 tests pass
- `just ready` → green (fmt, lint, test, deny)
- Manual: `cd /tmp/test-no-assay && cargo run -p assay-tui` → clean message, no panic, exits
- Manual: `cd <project-with-assay> && cargo run -p assay-tui` → dashboard with real milestone data visible

## Observability / Diagnostics

- Runtime signals: `Screen::NoProject` is the explicit failure mode for missing `.assay/`; renders a human-readable error string; exits on `q` — no hidden panic paths
- Inspection surfaces: `handle_event` returns `bool` (false = quit); each screen transition is a single enum-variant assignment — inspectable by reading `App.screen`
- Failure visibility: `milestone_scan` / `config::load` errors surfaced as `Screen::NoProject` with the error message rendered in the TUI; no silent failures
- Redaction constraints: none — no secrets displayed

## Integration Closure

- Upstream surfaces consumed: `assay_core::milestone::milestone_scan(assay_dir)`, `assay_core::config::load(root)`, `assay_types::{Milestone, MilestoneStatus, Config}`
- New wiring introduced in this slice: `main()` detects `.assay/`, calls `milestone_scan` + `config::load`, constructs `App`, enters event loop; `draw` dispatches to screen-specific free render fns; `handle_event` mutates `App.screen` on navigation keys
- What remains before the milestone is truly usable end-to-end: S02 (wizard rendering), S03 (chunk detail view), S04 (settings screen), S05 (help overlay + status bar + resize polish)

## Tasks

- [ ] **T01: Cargo.toml binary fix, App/Screen types, and run loop skeleton** `est:45m`
  - Why: The `[[bin]]` declaration, `App` struct, `Screen` enum, and `draw`/`handle_event` split are the structural foundation everything else builds on; S02 depends on `Screen::Wizard(WizardState)` existing; this task makes the codebase compile with the new shape
  - Files: `crates/assay-tui/Cargo.toml`, `crates/assay-tui/src/main.rs`
  - Do: (1) Add `[[bin]] name = "assay-tui" path = "src/main.rs"` before `[dependencies]` in Cargo.toml. (2) Rewrite `main.rs` entirely: define `WizardState { /* placeholder */ }` stub struct; define `Screen` enum with all six variants; define `App` struct per D089; split run loop into `fn draw(frame: &mut Frame, app: &App)` (match on `app.screen`, render placeholder for all screens initially) and `fn handle_event(app: &mut App, event: Event) -> bool` (return false on `q`, `Esc` goes to Dashboard, `Enter` on Dashboard goes to `MilestoneDetail`). (3) In `main()`: call `color_eyre::install()`, then `ratatui::init()` — do NOT add a manual `std::panic::set_hook` block. (4) Verify the crate compiles: `cargo check -p assay-tui`
  - Verify: `cargo check -p assay-tui` exits 0; `cargo build -p assay-tui` produces `target/debug/assay-tui`; `cargo build -p assay-cli` still produces `target/debug/assay`
  - Done when: `cargo build -p assay-tui && cargo build -p assay-cli` both succeed; binary names do not collide; `Screen::Wizard(WizardState)` variant compiles

- [ ] **T02: Dashboard rendering with real milestone data** `est:60m`
  - Why: Populates the dashboard with live data from `milestone_scan` and `config::load`; makes the slice demo visible — a developer launching the TUI sees actual milestone names and progress
  - Files: `crates/assay-tui/src/main.rs`
  - Do: (1) In `main()` after `ratatui::init()`: detect `.assay/` with `std::env::current_dir()?.join(".assay")`; if absent, set `App.screen = Screen::NoProject`; if present, call `milestone_scan(&assay_dir)` (takes `.assay/` dir) and `config::load(&project_root)` (takes project root); store results in `App`. (2) Implement `draw_dashboard(frame, app)` free fn: render a `Block::bordered().title("Dashboard")` containing a `List` of milestone items. Each item: `"{name}  [{status_badge}]  {completed}/{total}"` where `status_badge` is a match on `MilestoneStatus` producing `"Draft"/"Active"/"Verify"/"Done"`, and `{completed}/{total}` comes from `milestone.completed_chunks.len()` / `milestone.chunks.len()`. Use `render_stateful_widget(list, area, &mut app.list_state)`. (3) Implement `draw_no_project(frame)`: centered `Paragraph` with message `"Not an Assay project — run `assay init` first"`. (4) Wire `draw_dashboard` and `draw_no_project` into the `draw` fn's `Screen::Dashboard` and `Screen::NoProject` arms. (5) Path contract is critical: `milestone_scan(&assay_dir)` takes `.assay/` dir; `config::load(&project_root)` takes the project root (parent of `.assay/`)
  - Verify: `cargo build -p assay-tui` passes; manual: `cargo run -p assay-tui` from `assay` repo root (which has `.assay/` but no milestones) shows empty dashboard with "Dashboard" title; milestone names appear if any exist
  - Done when: Dashboard renders milestone list from real `milestone_scan` output; config load result stored in `App.config`; `Screen::NoProject` renders the clean message

- [ ] **T03: Navigation, empty-state guard, and unit tests** `est:60m`
  - Why: Completes the slice's interactive behavior (↑↓, `Enter`, `Esc`, `q`) and proves the state transitions are correct via automated tests; the empty-list guard prevents a latent panic
  - Files: `crates/assay-tui/src/main.rs`, `crates/assay-tui/tests/app_state.rs`
  - Do: (1) In `handle_event`, implement ↑↓ arrow key handlers: `KeyCode::Up` — decrement selection with wrap (if selection is 0 or None, wrap to last); `KeyCode::Down` — increment selection with wrap; guard `if milestones.is_empty()` and skip selection changes. (2) `Screen::MilestoneDetail` stub: render a `Paragraph` with `"Milestone detail — coming in S03"` and the selected milestone slug if derivable from `App.list_state`. (3) In `draw_dashboard`: guard `if app.milestones.is_empty()` → render a `Paragraph` with `"No milestones — press n to create one"` instead of rendering the `List` widget; this prevents the `ListState` panic. (4) `Esc` in `handle_event`: from any screen other than Dashboard, set `screen = Screen::Dashboard`. (5) Create `crates/assay-tui/tests/app_state.rs` with the following tests (use `TempDir` + `milestone_save` to create fixtures where needed): `navigate_down_increments_selection`, `navigate_up_wraps_to_last`, `navigate_down_wraps_to_first`, `quit_returns_false_from_dashboard`, `enter_on_dashboard_transitions_to_milestone_detail`, `esc_returns_to_dashboard_from_milestone_detail`, `empty_milestones_does_not_change_list_state`. Each test constructs `App` directly with known state and calls `handle_event` with a synthetic `Event::Key(KeyEvent)` — no terminal required. (6) Run `just ready` and fix any lint/fmt issues
  - Verify: `cargo test -p assay-tui 2>&1 | grep 'test result'` shows 7+ tests pass, 0 failed; `just ready` exits 0
  - Done when: All tests in `app_state.rs` pass; `just ready` green; ↑↓/Enter/Esc/q behavior verified by test; empty milestones produces no panic

## Files Likely Touched

- `crates/assay-tui/Cargo.toml` — add `[[bin]]` section
- `crates/assay-tui/src/lib.rs` — new; `App`, `Screen`, `WizardState`, `draw`, `handle_event`, `run` as pub items
- `crates/assay-tui/src/main.rs` — thinned to entry point; data loading moves here
- `crates/assay-tui/tests/app_state.rs` — new; unit tests for state transitions
