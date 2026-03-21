# S01: App Scaffold, Dashboard, and Binary Fix — Research

**Researched:** 2026-03-20
**Domain:** Ratatui 0.30, assay-core milestone/config APIs, Cargo binary naming
**Confidence:** HIGH

## Summary

The current `assay-tui` stub (42 lines, `crates/assay-tui/src/main.rs`) already uses correct Ratatui 0.30 patterns: `ratatui::init()`, `ratatui::restore()`, `DefaultTerminal`, `Layout`/`Constraint`, `Paragraph`, and crossterm event polling. The stub compiles and runs today. **The binary name issue is already resolved by default**: Cargo uses the package name (`assay-tui`) as the default binary name when no `[[bin]]` section is declared, so `cargo build -p assay-tui` already produces `target/debug/assay-tui`. Adding an explicit `[[bin]] name = "assay-tui" path = "src/main.rs"` entry to `Cargo.toml` is still the right call per D088 — it makes the intent unambiguous and mirrors `assay-cli`'s explicit pattern — but it is not blocking anything.

The `assay-core` data APIs needed for the dashboard are stable, tested, and require no new abstractions. `milestone_scan(assay_dir)` returns `Ok(vec![])` for a missing milestones directory (no panic, no error). `config::load(root)` returns `Err(AssayError::Io)` when `config.toml` is missing — the TUI must handle this by detecting the absence of `.assay/` before calling `load`. The path argument contract is a common confusion point: `milestone_scan` takes the `.assay/` directory directly, while `config::load` takes the project root (parent of `.assay/`).

The main S01 deliverable is replacing the 42-line stub with a real `App` struct, `Screen` enum, and `draw`/`handle_event` split, plus a working dashboard that renders a `List` of milestones loaded from `milestone_scan`. Three distinct areas require careful engineering: (1) the no-project guard that shows a clean "Not an Assay project" message and exits without panic when `.assay/` is absent; (2) the `ListState` empty-list guard that avoids a panic when `milestones` is empty; and (3) the panic hook — the stub installs a manual one, but `ratatui::init()` already installs its own hook internally (it calls `set_panic_hook()`), so the stub's manual hook is redundant and should be removed in the rewrite.

## Recommendation

Replace `main.rs` with an `App` struct + `Screen` enum. Keep data loading synchronous and on navigation transitions only (D091). Load milestones in `main()` after detecting `.assay/`, not inside `terminal.draw()`. Split into three modules if file length exceeds ~250 lines, otherwise keep everything in `main.rs` for simplicity. No new crate dependencies needed for S01.

Task decomposition:
- **T01**: Cargo.toml `[[bin]]` fix + `App`/`Screen` scaffold + run loop split (`draw` / `handle_event`)
- **T02**: Dashboard render with real `milestone_scan` + `config::load` data + `List`/`ListState` layout
- **T03**: No-project guard, empty-state rendering, ↑↓ navigation, `q` quit, `Enter`/`Esc` screen transition stubs, unit tests for state transitions

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Terminal init / panic hook / raw mode | `ratatui::init()` | Already used in stub; installs panic hook, enables raw mode, enters alternate screen in one call — do NOT add a second manual panic hook |
| Scrollable list with selection highlight | `ratatui::widgets::List` + `ListState` | `render_stateful_widget(list, area, &mut state)` — handles scroll offset and highlight automatically; stateful widget built in |
| Block borders and titles | `ratatui::widgets::Block` | `Block::bordered().title("Milestones")` — wraps any widget in one chain |
| Multi-area layout | `Layout::vertical/horizontal` with `Constraint` | Already imported in the stub; `Constraint::Fill(1)` / `Constraint::Length(n)` covers all S01 layout needs |
| Milestone scan | `assay_core::milestone::milestone_scan(assay_dir)` | Tested, atomic-read, returns `Vec<Milestone>` sorted by slug; returns `Ok(vec![])` for missing dir |
| Config load | `assay_core::config::load(root)` | Handles TOML parse + validation; returns structured `AssayError` on failure |
| Chunk progress computation | `Milestone.completed_chunks.len()` / `Milestone.chunks.len()` | Both fields are plain `Vec<String>` and `Vec<ChunkRef>` — no helper needed; compute inline |
| Status badge string | `MilestoneStatus` `#[serde(rename_all = "snake_case")]` | `format!("{:?}", status)` or match directly on the `MilestoneStatus` enum variants |

## Existing Code and Patterns

- `crates/assay-tui/src/main.rs` — 42-line stub. Replace entirely. Preserve the `ratatui::init()` / `ratatui::restore()` pattern. Remove the manual `std::panic::set_hook` block — `ratatui::init()` already installs its own panic hook internally via `set_panic_hook()` in `ratatui::init` module.
- `crates/assay-tui/Cargo.toml` — No `[[bin]]` section. Add `[[bin]] name = "assay-tui" path = "src/main.rs"` per D088. No other dependencies needed for S01 (all needed crates already present: `assay-core`, `ratatui`, `crossterm`, `color-eyre`).
- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan(assay_dir: &Path) -> Result<Vec<Milestone>>`. Takes `.assay/` dir. Returns sorted `Vec<Milestone>`. Returns `Ok(vec![])` when directory is absent. Use for dashboard data load.
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status(assay_dir)` returns `Option<CycleStatus>` with active milestone slug + phase + chunk progress. Useful for a status bar or header badge in S05, not required for S01.
- `crates/assay-core/src/config/mod.rs` — `load(root: &Path) -> Result<Config>`. Takes project root (parent of `.assay/`). Returns `Err(AssayError::Io)` if `config.toml` is missing. The TUI must guard: if `.assay/` does not exist, skip `load` entirely and go to `NoProject` screen.
- `crates/assay-types/src/milestone.rs` — `Milestone { slug, name, status: MilestoneStatus, chunks: Vec<ChunkRef>, completed_chunks: Vec<String>, ... }`. `MilestoneStatus` is `Draft | InProgress | Verify | Complete` with `#[default] = Draft`.
- `crates/assay-types/src/lib.rs` — `Config` struct. No `provider` field yet (added in S04). Fields: `project_name`, `specs_dir`, `gates: Option<GatesConfig>`, `guard`, `worktree`, `sessions`. Has `deny_unknown_fields` — do not pass unknown fields when constructing test configs.
- `crates/assay-core/tests/milestone_io.rs` — Reference for how to create `Milestone` test fixtures. Use `TempDir` + `milestone_save` to create fixture data for TUI unit tests.

## Constraints

- **D001 / D089**: Free functions only — `draw(frame, app)` is a free `fn`, not a `Widget` impl on `App`. No trait objects.
- **D091**: Load data in `handle_event` on navigation transitions, not inside `terminal.draw()`. For S01, load all milestone data once at startup (in `main()` before the event loop) and reload on `Refresh`. No background thread.
- **D007**: `assay-core` is sync. All data reads are `std::fs`. Do not introduce tokio in `assay-tui` for S01.
- **Cargo.toml**: The `[[bin]]` section must come before `[dependencies]` in TOML. Current `assay-tui/Cargo.toml` has no `[[bin]]` — add it; the build already works because Cargo defaults the binary name to the package name (`assay-tui`).
- **No `assay-tui` test infrastructure yet**: The `tests/` directory does not exist. Create `crates/assay-tui/tests/app_state.rs` as the first unit test file.

## Common Pitfalls

- **`config::load` panics on missing `.assay/`** — Actually returns `Err`, not panics, but calling it when `.assay/` doesn't exist produces a confusing "reading config" IO error. **Fix**: check `project_root.join(".assay").exists()` before calling `load`. If absent, render `Screen::NoProject` and exit cleanly. Do not call `milestone_scan` or `config::load` when `.assay/` is absent.

- **Path contract confusion: `assay_dir` vs `root`** — `milestone_scan(assay_dir)` takes `.assay/` directly; `config::load(root)` takes the project root (parent of `.assay/`). Getting this wrong produces "file not found" errors with confusing paths. Canonical pattern: `let assay_dir = project_root.join(".assay"); let milestones = milestone_scan(&assay_dir)?; let config = config::load(&project_root)?;`

- **`ListState` selection guard on empty lists** — `render_stateful_widget` with a `ListState` that has a stale selection index when the list is empty causes the widget to display nothing instead of an empty-state placeholder. **Fix**: guard with `if milestones.is_empty() { render_empty_placeholder(frame, area) }` before attempting to render the List widget. Reset `ListState` to `ListState::default()` (selection = `None`) when milestones reload.

- **Redundant panic hook** — The existing stub calls `std::panic::take_hook()` + `std::panic::set_hook()` manually. `ratatui::init()` already does this internally. The new `main.rs` should call `ratatui::init()` and NOT install an additional manual hook — doing so would replace the one ratatui installed, which could prevent terminal restoration on panic.

- **`project_root` detection** — The TUI should detect the project root from the current working directory using the canonical `.assay/` probe: walk up from `std::env::current_dir()` looking for `.assay/`. For S01, just use `current_dir()` directly — walking up is an S05 polish item.

- **`MilestoneStatus` display** — `MilestoneStatus` implements `Debug` but not `Display`. Use a `match` arm to produce badge strings: `Draft → "Draft"`, `InProgress → "Active"`, etc. Do not call `format!("{:?}", status)` in rendered output — it produces `"Draft"` / `"InProgress"` which is fine for S01 but fragile.

## Open Risks

- **No `.assay/` in the assay repo itself** (confirmed: `.assay/` exists but has no `milestones/` dir). The TUI integration test will need `TempDir`-based fixtures; it cannot rely on the project's own `.assay/` for milestone data.
- **`color-eyre` error hook interacts with ratatui panic hook** — The stub calls `color_eyre::install()` before `ratatui::init()`. The correct order is: install color-eyre first, then call `ratatui::init()` (which installs its panic hook last, ensuring terminal restore happens before color-eyre's hook). The current stub has this in the correct order — preserve it.
- **S02 assumes `Screen::Wizard(WizardState)` variant exists in S01** — The `WizardState` struct lives in `assay-tui`, so this variant cannot be added until S01 defines the `WizardState` type. S01 must define a stub `WizardState { /* placeholder */ }` and include the `Screen::Wizard(WizardState)` variant even if the wizard screen isn't rendered yet.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | (searched) | No relevant skill in `<available_skills>`; network search unavailable |

## Sources

- Codebase: `crates/assay-tui/src/main.rs` — current stub; init/restore/panic-hook pattern; crossterm event loop (HIGH confidence)
- Ratatui 0.30 init module: `~/.cargo/registry/src/.../ratatui-0.30.0/src/init.rs` — `ratatui::init()` installs panic hook internally via `set_panic_hook()`; `DefaultTerminal` = `Terminal<CrosstermBackend<Stdout>>` (HIGH confidence)
- Ratatui widgets: `~/.cargo/registry/src/.../ratatui-widgets-0.3.0/src/list.rs` — `List` + `ListState` stateful widget; `render_stateful_widget(list, area, &mut state)` API (HIGH confidence)
- Codebase: `crates/assay-core/src/milestone/mod.rs` — `milestone_scan(assay_dir)` returns `Ok(vec![])` for missing dir (HIGH confidence)
- Codebase: `crates/assay-core/src/config/mod.rs` — `load(root)` returns `Err(AssayError::Io)` for missing config.toml (HIGH confidence)
- Codebase: `crates/assay-types/src/milestone.rs` — `Milestone`, `ChunkRef`, `MilestoneStatus` types (HIGH confidence)
- Codebase: `crates/assay-tui/Cargo.toml` — no `[[bin]]` section; Cargo default produces `assay-tui` binary (confirmed by `ls target/debug/assay-tui`) (HIGH confidence)
