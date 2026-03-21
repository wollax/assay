---
estimated_steps: 5
estimated_files: 3
---

# T01: Cargo.toml binary fix, App/Screen types, and run loop skeleton

**Slice:** S01 — App Scaffold, Dashboard, and Binary Fix
**Milestone:** M006

## Description

Replace the 42-line stub's ad-hoc structure with the real `App`/`Screen` type hierarchy that the rest of M006 builds on. Adds the `[[bin]] name = "assay-tui"` declaration to resolve the binary naming ambiguity (D088), removes the redundant manual panic hook, and splits the run loop into `draw(frame, &App)` + `handle_event(&mut App, Event) -> bool` free functions per D089 and D001. Defines the `WizardState` stub so `Screen::Wizard(WizardState)` compiles (required by S02). All screen arms render placeholder text at this stage — real content comes in T02 and T03.

## Steps

1. Open `crates/assay-tui/Cargo.toml`. Add `[[bin]] name = "assay-tui" path = "src/main.rs"` before the `[dependencies]` section. This makes the binary name explicit per D088.

2. Create `crates/assay-tui/src/lib.rs` with the public types and logic that integration tests (in `tests/`) need to import. Binary crates in Rust cannot export items from `main.rs` to `tests/` — the standard pattern is a thin `main.rs` + a `lib.rs` that holds all logic. Put the following in `lib.rs`:
   - Imports: `assay_core::config::Config`, `assay_types::milestone::Milestone`, `crossterm::event::{Event, KeyCode}`, `ratatui::{DefaultTerminal, Frame, widgets::{Block, List, ListState, Paragraph}}`, `std::path::PathBuf`
   - `#[derive(Default)] pub struct WizardState { /* placeholder — real impl in S02 */ }`
   - `pub enum Screen { Dashboard, MilestoneDetail, ChunkDetail, Wizard(WizardState), Settings, NoProject }` — `#[allow(dead_code)]` on unused variants
   - `pub struct App { pub screen: Screen, pub milestones: Vec<Milestone>, pub list_state: ListState, pub project_root: Option<PathBuf>, pub config: Option<Config>, pub show_help: bool }`
   - `pub fn draw(frame: &mut Frame, app: &mut App)` — match on `app.screen`; all arms render a centered `Paragraph` with the screen name as placeholder text
   - `pub fn handle_event(app: &mut App, event: Event) -> bool` — `q` (and `Esc` on Dashboard) returns `false`; all other events return `true`
   - `pub fn run(app: &mut App, mut terminal: DefaultTerminal) -> color_eyre::Result<()>` — loop: `terminal.draw(|f| draw(f, app))?`; read event; if `!handle_event(app, event) { break }`

3. Rewrite `crates/assay-tui/src/main.rs` as a thin entry point that delegates to `lib.rs`:
   ```rust
   fn main() -> color_eyre::Result<()> {
       color_eyre::install()?;
       let terminal = ratatui::init();
       let mut app = assay_tui::App { screen: assay_tui::Screen::Dashboard, milestones: vec![], list_state: ratatui::widgets::ListState::default(), project_root: None, config: None, show_help: false };
       let result = assay_tui::run(&mut app, terminal);
       ratatui::restore();
       result
   }
   ```
   Do NOT add a `std::panic::set_hook` block — `ratatui::init()` installs its own panic hook.

4. `color_eyre::install()` must come before `ratatui::init()` per the existing stub's order.

5. Run `cargo check -p assay-tui` to verify the crate compiles cleanly; fix any type errors. Run `cargo build -p assay-tui && cargo build -p assay-cli` to confirm both binaries build without naming collision.

## Must-Haves

- [ ] `[[bin]] name = "assay-tui" path = "src/main.rs"` present in `crates/assay-tui/Cargo.toml` before `[dependencies]`
- [ ] `WizardState` struct defined and `Screen::Wizard(WizardState)` variant compiles
- [ ] All six `Screen` variants present: `Dashboard`, `MilestoneDetail`, `ChunkDetail`, `Wizard(WizardState)`, `Settings`, `NoProject`
- [ ] `App` struct has all six fields from D089: `screen`, `milestones`, `list_state`, `project_root`, `config`, `show_help`
- [ ] `draw` is a free `fn`, not a trait impl on `App`
- [ ] `handle_event` is a free `fn` that returns `bool`
- [ ] No `std::panic::set_hook` or `std::panic::take_hook` call in `main.rs`
- [ ] `cargo build -p assay-tui` → `target/debug/assay-tui` exists
- [ ] `cargo build -p assay-cli` → `target/debug/assay` exists; no binary name collision

## Verification

- `cargo build -p assay-tui && ls target/debug/assay-tui` → file exists
- `cargo build -p assay-cli && ls target/debug/assay` → file exists; `target/debug/assay` ≠ `target/debug/assay-tui`
- `cargo check -p assay-tui 2>&1 | grep -c error` → 0
- `grep '\\[\\[bin\\]\\]' crates/assay-tui/Cargo.toml` → matches `[[bin]]`
- `grep -c 'set_hook' crates/assay-tui/src/main.rs` → 0

## Observability Impact

- Signals added/changed: `handle_event` returning `bool` is the explicit quit signal — false = terminate, true = continue; this is the primary control-flow observable
- How a future agent inspects this: read `App.screen` to determine current screen; `App.milestones.len()` for data load status
- Failure state exposed: if `cargo build` fails, the error is a compiler diagnostic — no hidden state

## Inputs

- `crates/assay-tui/src/main.rs` — existing 42-line stub; will be replaced with thin entry point
- `crates/assay-tui/Cargo.toml` — missing `[[bin]]` section; needs addition
- Research: `Screen` enum variants and `App` field list from S01-RESEARCH.md and D089

## Expected Output

- `crates/assay-tui/Cargo.toml` — `[[bin]] name = "assay-tui"` added
- `crates/assay-tui/src/lib.rs` — new; contains `App`, `Screen`, `WizardState`, `draw`, `handle_event`, `run` as pub items
- `crates/assay-tui/src/main.rs` — thin entry point; delegates to `assay_tui::run`
- `target/debug/assay-tui` binary produced by `cargo build -p assay-tui`
