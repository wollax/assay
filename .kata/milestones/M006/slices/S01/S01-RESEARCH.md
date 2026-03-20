# S01: App Scaffold, Dashboard, and Binary Fix — Research

**Researched:** 2026-03-20
**Domain:** Ratatui 0.30, Rust TUI architecture, assay-core data APIs
**Confidence:** HIGH

## Summary

S01 has two distinct sub-problems: (1) a one-line fix (`[[bin]]` declaration in `assay-tui/Cargo.toml`) that unblocks all subsequent TUI work, and (2) replacing the 42-line stub with a real `App` struct / `Screen` enum / dashboard rendering stack. Both are well-bounded and straightforward. The primary risk is getting the no-project guard right — `config::load(root)` returns `AssayError::Io` when `config.toml` is missing; the TUI must detect the absence of `.assay/` before any data read and show a clean splash, not propagate an error.

The existing stub already uses the correct Ratatui 0.30 primitives (`ratatui::init()`, `ratatui::restore()`, `DefaultTerminal`, panic hook pattern). The replacement preserves all of this exactly. `milestone_scan(assay_dir)` returns `Ok(vec![])` when `milestones/` doesn't exist, so the empty-milestones empty-state is safe by default. The gate pass/fail column in the dashboard requires calling `history::list` + `history::load` per chunk — this is O(file count) and can be done synchronously on dashboard load for S01. If profiling shows lag, background loading can be added in S05 polish.

The architecture is locked by D089 (App struct + Screen enum), D090 (WizardState in Screen::Wizard), D091 (sync data loading on navigation), and D088 (binary name is `assay-tui`). No architectural decisions remain open for S01.

## Recommendation

Execute in this order: (1) Add `[[bin]] name = "assay-tui" path = "src/main.rs"` to `crates/assay-tui/Cargo.toml` and verify `cargo build -p assay-tui` produces `target/debug/assay-tui`. (2) Define `App`, `Screen`, and all types in a new `app.rs` module, using free render functions (`draw_dashboard`, `draw_no_project`, `draw_no_milestones`) per D001. (3) Implement the event loop with `event::poll(Duration::from_millis(250))` + `event::read()` for responsiveness — the blocking `event::read()` in the stub is fine for `q` only but a polling loop lets you load data on the first frame. (4) Load `milestone_scan` + `config::load` at startup (in `run()`, not inside `terminal.draw()`). (5) Render the dashboard list with `render_stateful_widget` + `ListState`. (6) Add unit tests for `App::new()`, `handle_event` state transitions (up/down/q), and the empty-state rendering paths.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Terminal init + panic hook + raw mode | `ratatui::init()` + `std::panic::set_hook` wrapping `ratatui::restore()` | Already in the stub; handles alternate screen, raw mode, panic restore — preserve verbatim |
| Scrollable list with keyboard selection | `ratatui::widgets::List` + `ListState::default()` + `render_stateful_widget` | `ListState` has `select_next()` / `select_previous()` / `with_selected(Some(0))`; auto-scrolls; do not hand-roll |
| Block border + title | `ratatui::widgets::Block::bordered().title("...")` | One method chain; wraps any widget |
| Horizontal/vertical layout | `Layout::vertical([Constraint::Length(3), Constraint::Fill(1), Constraint::Length(1)])` | Already imported in stub; handles fixed header + fill body + fixed status bar |
| Milestone scan | `assay_core::milestone::milestone_scan(assay_dir)` | Returns `Vec<Milestone>` sorted by slug; returns `Ok(vec![])` if `milestones/` absent — safe to call always |
| Config load | `assay_core::config::load(project_root)` | `project_root` = parent of `.assay/`. Returns `AssayError::Io` if `config.toml` missing — handle this as the "no project" signal |
| Cycle status | `assay_core::milestone::cycle_status(assay_dir)` | Returns `Option<CycleStatus>` with active milestone slug, phase, and progress counts — use for header badge |
| Gate history (for pass/fail column) | `assay_core::history::list(assay_dir, spec_name)` + `history::load(assay_dir, spec_name, run_id)` | `list()` returns chronologically sorted run IDs; last entry is most recent; `load()` returns `GateRunRecord` with `summary.passed` / `summary.failed` |

## Existing Code and Patterns

- `crates/assay-tui/src/main.rs` — 42-line stub; replace entirely, preserve: `color_eyre::install()`, `std::panic::set_hook` calling `ratatui::restore()`, `ratatui::init()` / `ratatui::restore()` wrapping `run()`. The `run(mut terminal: DefaultTerminal) -> color_eyre::Result<()>` signature stays.
- `crates/assay-tui/Cargo.toml` — Missing `[[bin]]` declaration. Add before any other work: `[[bin]]\nname = "assay-tui"\npath = "src/main.rs"`. No other dep changes needed for S01 — `ratatui`, `crossterm`, `assay-core`, `color-eyre` are already there.
- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan(assay_dir: &Path)` public function; reads `<assay_dir>/milestones/`; returns sorted `Vec<Milestone>`; safe on missing dir.
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status(assay_dir: &Path) -> Result<Option<CycleStatus>>`. `CycleStatus { milestone_slug, milestone_name, phase: MilestoneStatus, active_chunk_slug: Option<String>, completed_count, total_count }`. Use for dashboard header active-milestone badge.
- `crates/assay-core/src/config/mod.rs` — `load(root: &Path) -> Result<Config>` where `root` is project root (parent of `.assay/`). Returns `AssayError::Io` on missing file — this is the no-project detection signal.
- `crates/assay-types/src/milestone.rs` — `Milestone { slug, name, status: MilestoneStatus, chunks: Vec<ChunkRef>, completed_chunks: Vec<String>, ... }`. `MilestoneStatus` is `Draft | InProgress | Verify | Complete` (snake_case in serde). `ChunkRef { slug, order }`.
- `crates/assay-types/src/gate_run.rs` — `GateRunRecord { summary: GateRunSummary, ... }`. `GateRunSummary { passed, failed, skipped, enforcement: EnforcementSummary, ... }`. Use `summary.passed` / `summary.failed` for the gate column.
- `crates/assay-core/tests/cycle.rs` — Pattern for test helpers: `make_assay_dir(tmp)`, `create_passing_spec(assay_dir, slug)`, `make_milestone_with_status(slug, status, chunks)`. Reuse in TUI unit tests.
- `crates/assay-cli/Cargo.toml` — `[[bin]] name = "assay" path = "src/main.rs"`. Confirms the TUI must NOT use `name = "assay"`.

## Constraints

- **Binary name**: `assay-cli` owns `name = "assay"`. TUI must use `name = "assay-tui"` (D088). This is the first line in the implementation task.
- **Zero traits** (D001): `App` struct methods + free render functions. No `Widget` trait impls on app state. The exception is self-contained pure display types — not needed for S01.
- **Sync data loading** (D007, D091): All `milestone_scan`, `config::load`, `cycle_status` calls are `std::fs` sync. Load on navigation transitions (inside `handle_event`), not inside `terminal.draw()`. Do not add tokio.
- **`Config` has `deny_unknown_fields`**: Do not add any fields to `Config` in S01 — that's S04. Just load and display.
- **No `.assay/` guard**: `config::load()` fails with `AssayError::Io` when config.toml missing. Detect `.assay/` directory existence in `run()` before any data calls. If absent, show `Screen::NoProject` and exit (or just display the message and wait for `q`).
- **`ListState` bounds**: `ListState::selected()` can be `None` on empty lists. Guard all `if let Some(i) = list_state.selected()` blocks. On empty milestone list, show explicit "No milestones — run `assay plan`" placeholder paragraph.

## Common Pitfalls

- **Blocking `event::read()` in a multi-screen app** — The stub uses `event::read()` which blocks indefinitely until a key. For S01 this is acceptable (data is loaded once at startup), but the recommended pattern is `event::poll(Duration::from_millis(250)); if event::poll(...)? { event::read()? }` so the loop can refresh data on navigation without blocking.
- **Calling data reads inside `terminal.draw()`** — `terminal.draw()` holds a mutable terminal borrow. Any `milestone_scan` or `history::load` call inside it blocks the render thread. Load all data in `run()` or `handle_event()`, not in `draw()`.
- **`ListState` with empty list panics** — `render_stateful_widget` with `ListState::selected() = Some(0)` on a zero-item list does not panic, but selection-dependent logic (Enter key handler) must guard `if items.is_empty()`. Reset selection to `None` when the data list changes.
- **Missing `[[bin]]` declaration** — Without it, `cargo build -p assay-tui` produces no binary. The crate compiles as a library. This is the first change to make; verify with `cargo build -p assay-tui && ls target/debug/assay-tui`.
- **`config::load` error on missing file is `AssayError::Io`, not a panic** — Pattern: `match config::load(project_root) { Ok(c) => Some(c), Err(_) => None }`. Then if `project_root.join(".assay").exists()` is false, show `Screen::NoProject`. If `.assay/` exists but `config.toml` is missing or invalid, show the dashboard anyway with `config: None` (graceful degradation).
- **Gate history loading latency** — Calling `history::list` + `history::load` per chunk per milestone on dashboard load is O(milestones × chunks × history_files). For a typical project (2-5 milestones, 3-7 chunks each, 1-10 history records each) this is fast. But the implementation must guard against `history::list` returning `Err` (no history dir) gracefully — show `pending` badge, not an error.

## Open Risks

- **Terminal resize during rendering** — Ratatui handles resize automatically on the next frame if the event loop continues. No special handling needed in S01; the next `terminal.draw()` call after a resize event gets the new dimensions. Add `Event::Resize` to the event match to explicitly ignore it (not return early as a quit signal).
- **Gate history for dashboard** — The roadmap says "gate pass/fail counts per milestone" is required. Computing this synchronously at dashboard load requires iterating all chunk history. It is safe but adds latency proportional to history file count. If this proves slow in S01 integration testing, the gate column can be `N/A` on initial render and populated lazily. The risk is overestimating latency.
- **`assay_dir` vs `project_root` in function signatures** — `milestone_scan` takes `assay_dir` (path to `.assay/`), while `config::load` takes `project_root` (parent of `.assay/`). `cycle_status` takes `assay_dir`. This asymmetry is already established in assay-core — `App` must store `project_root: PathBuf` and derive `assay_dir` as `project_root.join(".assay")`.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | (searched via available_skills) | None found — inline research used instead |

## Sources

- `crates/assay-tui/src/main.rs` — Current stub; panic hook + init/restore pattern to preserve (HIGH confidence)
- `crates/assay-tui/Cargo.toml` — Confirmed: no `[[bin]]` declaration; `assay-core`, `ratatui`, `crossterm` already in deps (HIGH confidence)
- `crates/assay-cli/Cargo.toml` — Confirmed: `[[bin]] name = "assay"` — TUI must use a different name (HIGH confidence)
- `~/.cargo/registry/src/.../ratatui-widgets-0.3.0/src/list/state.rs` — `ListState` API: `Default`, `with_selected()`, `select_next()`, `select_previous()`, `selected() -> Option<usize>` (HIGH confidence)
- `~/.cargo/registry/src/.../ratatui-0.30.0/src/init.rs` — `init()`, `restore()`, `DefaultTerminal` type alias (HIGH confidence)
- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan(assay_dir)` public API surface (HIGH confidence)
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status(assay_dir) -> Result<Option<CycleStatus>>`, `CycleStatus` fields (HIGH confidence)
- `crates/assay-core/src/config/mod.rs` — `load(root)` takes project root; returns `AssayError::Io` on missing file (HIGH confidence)
- `crates/assay-types/src/milestone.rs` — `Milestone`, `ChunkRef`, `MilestoneStatus` types and serde conventions (HIGH confidence)
- `crates/assay-types/src/gate_run.rs` — `GateRunRecord`, `GateRunSummary.passed/failed` fields (HIGH confidence)
- `crates/assay-core/tests/cycle.rs` — Test helper patterns for milestone fixtures reusable in TUI unit tests (HIGH confidence)
