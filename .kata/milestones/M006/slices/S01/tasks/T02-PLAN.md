---
estimated_steps: 5
estimated_files: 1
---

# T02: Dashboard rendering with real milestone data

**Slice:** S01 — App Scaffold, Dashboard, and Binary Fix
**Milestone:** M006

## Description

Replaces the placeholder dashboard with a real dashboard: loads milestones via `milestone_scan` and config via `config::load` in `main()`, then renders a `List` of milestone entries (name, status badge, chunk progress fraction) using `render_stateful_widget`. Implements the no-project guard — if `.assay/` is absent, sets `App.screen = Screen::NoProject` and renders a clean exit message. Path contract is a known footgun: `milestone_scan` takes the `.assay/` directory directly; `config::load` takes the project root (parent of `.assay/`).

## Steps

1. Add imports needed for data loading: `assay_core::milestone::milestone_scan`, `assay_core::config`, `std::env::current_dir`. Add `assay_core::milestone` to the existing `assay_core` import tree.

2. Update `main()` to perform data loading before entering the run loop:
   ```
   let project_root = current_dir()?;
   let assay_dir = project_root.join(".assay");
   let (milestones, config, initial_screen) = if assay_dir.exists() {
       let milestones = milestone_scan(&assay_dir).unwrap_or_default();
       let config = config::load(&project_root).ok();
       (milestones, config, Screen::Dashboard)
   } else {
       (vec![], None, Screen::NoProject)
   };
   let mut app = App { screen: initial_screen, milestones, list_state: ListState::default(), project_root: Some(project_root), config, show_help: false };
   ```
   Note: `milestone_scan` returns `Ok(vec![])` for a missing milestones dir — no need to handle that case separately. If `milestone_scan` errors (e.g. corrupt file), use `unwrap_or_default()` to degrade gracefully.

3. Implement `fn draw_dashboard(frame: &mut Frame, app: &App)` as a free function:
   - Outer layout: `Layout::vertical([Constraint::Fill(1)])` (single area for S01; status bar added in S05)
   - Render `Block::bordered().title(" Assay Dashboard ")` around the list area
   - Build `List` items: for each milestone in `app.milestones`, format a `ListItem` with text `"{name}  [{badge}]  {done}/{total}"` where:
     - `badge` = `match status { MilestoneStatus::Draft => "Draft", MilestoneStatus::InProgress => "Active", MilestoneStatus::Verify => "Verify", MilestoneStatus::Complete => "Done" }`
     - `done` = `milestone.completed_chunks.len()`
     - `total` = `milestone.chunks.len()`
   - Use `render_stateful_widget(list, inner_area, &mut app.list_state)` — note `&mut app.list_state` requires `app` to be `&mut App` in this function or the `list_state` to be passed separately; prefer signature `fn draw_dashboard(frame: &mut Frame, app: &App, list_state: &mut ListState)` to keep `draw` taking `&App`
   - Highlight selected item with `List::new(items).highlight_style(Style::default().reversed())`

4. Implement `fn draw_no_project(frame: &mut Frame)`:
   - Render a centered `Paragraph` with text `"Not an Assay project — run \`assay init\` first"` styled `bold().red()`
   - Add a sub-line: `"Press q to quit"`

5. Wire both fns into the `draw` fn's `Screen::Dashboard` arm (passing `&mut app.list_state` explicitly) and `Screen::NoProject` arm. Update `draw`'s signature to take `&mut App` to allow passing `list_state` mutably if needed; alternatively, pull `list_state` from `App` before calling `draw_dashboard`. Either pattern is fine — pick the simpler one. Run `cargo build -p assay-tui` and fix errors.

## Must-Haves

- [ ] `main()` detects `.assay/` with `project_root.join(".assay").exists()` before calling any assay-core functions
- [ ] `milestone_scan` called with `assay_dir` (the `.assay/` path), NOT `project_root` — path contract is critical
- [ ] `config::load` called with `project_root` (parent of `.assay/`), NOT `assay_dir`
- [ ] No `.assay/` → `App.screen = Screen::NoProject` set at startup; `draw_no_project` renders clean message
- [ ] `draw_dashboard` renders a `List` with at least name, status badge, and progress fraction per milestone
- [ ] `render_stateful_widget` used for the `List` (not `render_widget`) so `ListState` selection works
- [ ] `status_badge` uses explicit `match` arms, not `format!("{:?}", status)` in rendered output
- [ ] `cargo build -p assay-tui` passes

## Verification

- `cargo build -p assay-tui 2>&1 | grep -c '^error'` → 0
- Manual: `cargo run -p assay-tui` from the `assay` repo root → dashboard renders with `Block` border and title; if `.assay/milestones/` has files they appear; no panic on empty milestones dir
- Manual: `cd /tmp && mkdir no-assay && cd no-assay && cargo run --manifest-path /path/to/assay/Cargo.toml -p assay-tui` → clean "Not an Assay project" message displayed; TUI doesn't panic; exits on `q`
- `grep -c 'milestone_scan' crates/assay-tui/src/main.rs` → ≥ 1 (confirms real data loading wired in)

## Observability Impact

- Signals added/changed: `App.milestones.is_empty()` distinguishes "no project" from "project has no milestones" — these are different states with different rendering
- How a future agent inspects this: `App.screen` holds `NoProject` when `.assay/` is absent; `App.milestones.len()` tells how many were loaded; `App.config.is_some()` tells whether config loaded successfully
- Failure state exposed: `milestone_scan` errors degrade to empty vec (not a panic); `config::load` errors degrade to `None` — both visible via `App` field inspection

## Inputs

- `crates/assay-tui/src/lib.rs` — T01 output with `App`, `Screen`, `draw`, `handle_event` scaffolded; `main.rs` thin entry point
- `assay_core::milestone::milestone_scan` — takes `&Path` (`.assay/` dir); returns `Result<Vec<Milestone>>`
- `assay_core::config::load` — takes `&Path` (project root); returns `Result<Config>`
- `assay_types::milestone::{Milestone, MilestoneStatus}` — `completed_chunks: Vec<String>`, `chunks: Vec<ChunkRef>`

## Expected Output

- `crates/assay-tui/src/lib.rs` — `draw_dashboard` and `draw_no_project` implemented; `draw` dispatches to them
- `crates/assay-tui/src/main.rs` — updated to perform real data loading in `main()` with path-contract-correct calls before constructing `App`
