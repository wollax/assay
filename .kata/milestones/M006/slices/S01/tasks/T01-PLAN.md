---
estimated_steps: 5
estimated_files: 3
---

# T01: Binary fix, `App`/`Screen` scaffold, and failing tests

**Slice:** S01 — App Scaffold, Dashboard, and Binary Fix
**Milestone:** M006

## Description

The `assay-tui` crate is currently missing a `[[bin]]` declaration, so `cargo build -p assay-tui` produces no binary. This is the highest-priority blocking defect (D088). This task fixes it, creates the foundational `App` + `Screen` types in a new `src/app.rs` module, updates `main.rs` to delegate to `app::run()`, and writes the test assertions that T02 must satisfy. Tests fail at the end of T01 (stubs return dummy values) — that is correct and expected.

## Steps

1. Open `crates/assay-tui/Cargo.toml` and add a `[[bin]]` section and `[dev-dependencies]` section:
   ```toml
   [dev-dependencies]
   tempfile.workspace = true
   ```
   Then add the `[[bin]]` section immediately before `[dependencies]`:
   ```toml
   [[bin]]
   name = "assay-tui"
   path = "src/main.rs"
   ```
   Verify `cargo build -p assay-tui && ls target/debug/assay-tui` exits 0 and `cargo build -p assay-cli && ls target/debug/assay` still exits 0.

2. Create `crates/assay-tui/src/app.rs` with:
   - Imports: `assay_core::config`, `assay_core::milestone`, `assay_types::{Config, Milestone}`, `ratatui::{DefaultTerminal, Frame, widgets::{Block, List, ListItem, ListState, Paragraph}, layout::{Constraint, Layout}}`, `crossterm::event::{self, Event, KeyCode, KeyEvent}`
   - `pub enum Screen { Dashboard, NoProject }`
   - `pub struct GateSummary { pub passed: u32, pub failed: u32 }` 
   - `pub struct App { pub screen: Screen, pub milestones: Vec<Milestone>, pub gate_data: Vec<(String, GateSummary)>, pub list_state: ListState, pub project_root: std::path::PathBuf, pub config: Option<Config> }`
   - Stub `impl App { pub fn new(project_root: std::path::PathBuf) -> Self { /* TODO */ App { screen: Screen::Dashboard, milestones: vec![], gate_data: vec![], list_state: ListState::default(), project_root, config: None } } }`
   - Stub `pub fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> { Ok(()) }`
   - Stub `pub fn handle_event(app: &mut App, event: &Event) -> bool { false }`
   - Stub `pub fn draw(frame: &mut Frame, app: &mut App) {}`

3. Update `crates/assay-tui/src/main.rs` to replace the inline `run()` loop with a call to `app::run(terminal)`. Preserve the panic hook, `ratatui::init()`, and `ratatui::restore()` exactly. Add `mod app;` at the top.

4. Add a `#[cfg(test)]` module at the bottom of `app.rs` with these tests:
   - `test_no_assay_dir_sets_no_project_screen`: create a tempdir without `.assay/`; call `App::new(tmp.path().to_path_buf())`; assert `matches!(app.screen, Screen::NoProject)`
   - `test_handle_event_q_returns_true`: create any `App`; call `handle_event(&mut app, &Event::Key(KeyEvent::from(KeyCode::Char('q'))))`; assert result is `true`
   - `test_handle_event_up_down_no_panic_on_empty`: create an `App` with empty `milestones`; call `handle_event` with `Up` and `Down`; assert both return `false` and no panic
   - `test_handle_event_down_moves_selection`: create an `App` with `milestones` containing 2 items and `list_state` selected at `Some(0)`; call `handle_event(Down)`; assert `list_state.selected() == Some(1)` (or wraps)

5. Run `cargo test -p assay-tui`. Expect some tests to fail (stubs). Expect `test_handle_event_q_returns_true` and `test_handle_event_up_down_no_panic_on_empty` to fail. That is correct — T02 makes them pass.

## Must-Haves

- [ ] `cargo build -p assay-tui && ls target/debug/assay-tui` exits 0
- [ ] `cargo build -p assay-cli && ls target/debug/assay` exits 0 (no collision)
- [ ] `App`, `Screen`, `GateSummary` types compile without errors
- [ ] `main.rs` delegates to `app::run()` and preserves panic hook + init/restore
- [ ] `#[cfg(test)]` module compiles and tests run (failures are expected at this stage)
- [ ] No `[[bin]] name = "assay"` added to assay-tui (that would collide with assay-cli)

## Verification

- `cargo build -p assay-tui && ls -la target/debug/assay-tui` — binary present
- `cargo build -p assay-cli && ls -la target/debug/assay` — cli binary present
- `cargo test -p assay-tui 2>&1 | grep -E "FAILED|error\[E"` — should show test failures but NOT compiler errors
- `cargo check --workspace` — entire workspace compiles

## Observability Impact

- Signals added/changed: `Screen` enum is the sole state signal; `Screen::NoProject` will render a visible error message (implemented in T02)
- How a future agent inspects this: `app.screen` variant tells you which render path was chosen; unit tests assert on `Screen::NoProject` transitions
- Failure state exposed: If `App::new()` is called on a missing `.assay/` dir, `Screen::NoProject` is the expected result — inspectable in tests and visible to the user in T02's render implementation

## Inputs

- `crates/assay-tui/src/main.rs` — 42-line stub; preserve panic hook + `ratatui::init()` / `ratatui::restore()` wrapping
- `crates/assay-tui/Cargo.toml` — add `[[bin]]`; add `tempfile.workspace = true` as dev-dependency
- `crates/assay-cli/Cargo.toml` — confirm `[[bin]] name = "assay"` to avoid collision

## Expected Output

- `crates/assay-tui/Cargo.toml` — has `[[bin]] name = "assay-tui"` and `tempfile` dev-dependency
- `crates/assay-tui/src/main.rs` — delegates to `app::run()`; preserved panic hook
- `crates/assay-tui/src/app.rs` — new file with `App`, `Screen`, `GateSummary` types, stubs, and failing `#[cfg(test)]` tests
- `target/debug/assay-tui` binary produced by `cargo build -p assay-tui`
