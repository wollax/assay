---
estimated_steps: 11
estimated_files: 2
---

# T02: Dashboard rendering, event loop, and gate data — all tests pass

**Slice:** S01 — App Scaffold, Dashboard, and Binary Fix
**Milestone:** M006

## Description

Implements all real behavior defined in the T01 scaffolding: `App::new()` loads live milestone data and gate history from `assay-core`; the polling event loop replaces the blocking `event::read()` stub; `handle_event()` handles `q`, arrow keys, and `Resize`; `draw()` dispatches to `draw_dashboard`, `draw_no_project`, and `draw_no_milestones`; the dashboard list shows each milestone's name, status badge, chunk fraction, and gate pass/fail column from real history files. All unit tests from T01 pass. `just ready` is green.

## Steps

1. Implement `App::new(project_root: PathBuf) -> Self`:
   - `let assay_dir = project_root.join(".assay");`
   - If `!assay_dir.exists()` → return `App { screen: Screen::NoProject, milestones: vec![], gate_data: vec![], list_state: ListState::default(), project_root, config: None }`
   - `let milestones = assay_core::milestone::milestone_scan(&assay_dir).unwrap_or_default();`
   - `let config = assay_core::config::load(&project_root).ok();`
   - `let gate_data = compute_gate_data(&assay_dir, &milestones);` (free function, see step 2)
   - `let list_state = if milestones.is_empty() { ListState::default() } else { ListState::default().with_selected(Some(0)) };`
   - Return `App { screen: Screen::Dashboard, milestones, gate_data, list_state, project_root, config }`

2. Implement `fn compute_gate_data(assay_dir: &Path, milestones: &[Milestone]) -> Vec<(String, GateSummary)>`:
   - For each milestone, iterate `milestone.chunks`; for each chunk, call `assay_core::history::list(assay_dir, &chunk.slug).unwrap_or_default()`; if list is non-empty, take the last run_id and call `assay_core::history::load(assay_dir, &chunk.slug, &run_id).ok()`; accumulate `summary.passed` and `summary.failed` across all chunks in the milestone
   - Return `vec![(milestone.slug.clone(), GateSummary { passed, failed })]` for each milestone
   - All errors are silently degraded to zero counts — no panic, no early return

3. Implement `pub fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()>`:
   - Detect `project_root` as `std::env::current_dir()?`
   - Construct `let mut app = App::new(project_root);`
   - Event loop: `loop { terminal.draw(|f| draw(f, &mut app))?; if event::poll(std::time::Duration::from_millis(250))? { let ev = event::read()?; if handle_event(&mut app, &ev) { break; } } }`
   - Return `Ok(())`

4. Implement `pub fn handle_event(app: &mut App, event: &Event) -> bool`:
   - Match `Event::Key(key)`:
     - `KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc → return true` (when screen is NoProject, Esc/q still quits)
     - `KeyCode::Up → if !app.milestones.is_empty() { app.list_state.select_previous(); }`
     - `KeyCode::Down → if !app.milestones.is_empty() { app.list_state.select_next(); }`
     - `KeyCode::Enter → {}` (screen transition placeholder for S02/S03)
   - Match `Event::Resize(..) → return false` (explicit ignore — don't quit on resize)
   - Default → `false`

5. Implement `pub fn draw(frame: &mut Frame, app: &mut App)`:
   - Match `app.screen`: `Screen::Dashboard → draw_dashboard(frame, app)`, `Screen::NoProject → draw_no_project(frame)`

6. Implement `fn draw_no_project(frame: &mut Frame)`:
   - Render a centered `Paragraph` with text `"Not an Assay project — run `assay init` first"` wrapped in a `Block::bordered().title("Assay")`
   - Use `Layout::vertical([Constraint::Fill(1)])` to fill the full frame

7. Implement `fn draw_no_milestones(frame: &mut Frame, area: ratatui::layout::Rect)`:
   - Render a `Paragraph` with `"No milestones — run `assay plan`"` centered in `area`

8. Implement `fn draw_dashboard(frame: &mut Frame, app: &mut App)`:
   - Layout: `Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)])` → header / body / footer areas
   - Header: `Paragraph::new("Assay Dashboard").bold()` in header area
   - Footer: `Paragraph::new(" q quit  ↑↓ navigate")` in footer area
   - Body: if `app.milestones.is_empty()` → call `draw_no_milestones(frame, body_area)` and return
   - Build `Vec<ListItem>` from `app.milestones` + `app.gate_data`: each item text = `format!("{:30} {:10} {:5} ✓{} ✗{}", name, status_badge(status), chunk_fraction, passed, failed)` where `chunk_fraction = "{}/{}".format(completed_chunks.len(), chunks.len())`
   - `status_badge(status)` returns `"[Draft]"` / `"[InProgress]"` / `"[Verify]"` / `"[Complete]"`
   - Build `List::new(items).highlight_symbol("▶ ").block(Block::bordered().title("Milestones"))`
   - `frame.render_stateful_widget(list, body_area, &mut app.list_state)`

9. Add one more test: `test_gate_data_loaded_from_history`: create a `TempDir`; call `assay_core::init::init_project(tmp.path())` or manually create `.assay/milestones/` dir; use `assay_core::milestone::milestone_save` to write a milestone with one chunk; use `assay_core::history::save_run` to write a gate run record with `passed: 2, failed: 1`; call `App::new(tmp.path())`; assert `gate_data` contains an entry for that milestone slug with `passed == 2, failed == 1`. (If `init_project` adds complexity, just `std::fs::create_dir_all(tmp.path().join(".assay/milestones"))` and write fixture files directly.)

10. Run `cargo test -p assay-tui`. All tests from T01 must now pass. Fix any failures before proceeding.

11. Run `just ready`. Fix any fmt/clippy/deny issues. Commit is not required here — T01 and T02 will be committed together.

## Must-Haves

- [ ] `cargo test -p assay-tui` — all tests pass (no failures)
- [ ] `test_no_assay_dir_sets_no_project_screen` passes: `App::new()` on a dir without `.assay/` → `Screen::NoProject`
- [ ] `test_handle_event_q_returns_true` passes: `handle_event(Char('q'))` → `true`
- [ ] `test_handle_event_up_down_no_panic_on_empty` passes: Up/Down on empty list → `false`, no panic
- [ ] `test_handle_event_down_moves_selection` passes: Down on 2-item list → selection advances
- [ ] `compute_gate_data` handles `history::list` returning `Err` (no history dir) → `GateSummary { passed: 0, failed: 0 }` (no panic)
- [ ] `draw_dashboard` renders with `render_stateful_widget` (not `render_widget`) so selection state is respected
- [ ] `just ready` passes

## Verification

- `cargo test -p assay-tui` — all tests green
- `cargo build -p assay-tui && ls -la target/debug/assay-tui` — binary present
- `just ready` — fmt/lint/test/deny all pass
- Manual: `cargo run -p assay-tui` in a project with `.assay/milestones/` (e.g. assay's own dev fixtures) shows list of milestones; `↑↓` moves selection highlight; `q` exits; `cargo run -p assay-tui` in a directory without `.assay/` shows "Not an Assay project" message; no panic in either case

## Observability Impact

- Signals added/changed: `Screen::NoProject` renders a human-readable "Not an Assay project" message to the terminal — the user immediately knows what went wrong without reading a log file
- How a future agent inspects this: `app.screen` + `app.milestones.len()` + `app.gate_data` are all inspectable in unit tests; test helpers from `crates/assay-core/tests/cycle.rs` (`make_assay_dir`, `make_milestone_with_status`) can construct fixture state for TUI tests
- Failure state exposed: Silent degradation on data load errors means TUI shows empty dashboard rather than crashing — a future agent sees the empty state and knows to check `.assay/` directory contents directly

## Inputs

- `crates/assay-tui/src/app.rs` — stub types from T01; `#[cfg(test)]` tests to satisfy
- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan(assay_dir)` API
- `crates/assay-core/src/milestone/cycle.rs` — `CycleStatus` fields (for potential header badge in S05)
- `crates/assay-core/src/config/mod.rs` — `load(project_root)` returns `Err` when config.toml absent
- `crates/assay-core/src/history/mod.rs` — `list(assay_dir, spec_name)` + `load(assay_dir, spec_name, run_id)` APIs
- `crates/assay-types/src/milestone.rs` — `Milestone`, `ChunkRef`, `MilestoneStatus` types
- `crates/assay-types/src/gate_run.rs` — `GateRunRecord.summary.passed / .failed` fields

## Expected Output

- `crates/assay-tui/src/app.rs` — fully implemented `App::new()`, `run()`, `handle_event()`, `draw()`, `draw_dashboard()`, `draw_no_project()`, `draw_no_milestones()`, `compute_gate_data()`, and passing `#[cfg(test)]` module
- `crates/assay-tui/src/main.rs` — unchanged from T01 (delegates to `app::run()`)
- `cargo test -p assay-tui` green
- `just ready` green
