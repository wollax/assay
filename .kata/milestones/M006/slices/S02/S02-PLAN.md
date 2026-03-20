# S02: In-TUI Authoring Wizard

**Goal:** Implement a multi-step Ratatui form (`WizardState` + `draw_wizard` + `handle_wizard_event`) that collects milestone name, description, chunk count, per-chunk names, and per-chunk criteria — then calls `create_from_inputs` and returns the user to a refreshed dashboard showing the new milestone.

**Demo:** After this: pressing `n` from the dashboard opens a multi-step form; completing it calls `create_from_inputs` and the new milestone appears in the dashboard list immediately; proven by an integration test that writes real files to a tempdir. This retires the wizard form complexity risk (highest-rated risk in M006).

## Must-Haves

- `WizardState` struct in `assay-tui` with `step`, `fields: Vec<Vec<String>>`, `cursor`, `chunk_count`, `error: Option<String>`
- `StepKind` enum with `current_step_kind()` helper to avoid raw index arithmetic
- `handle_wizard_event(state: &mut WizardState, event: KeyEvent) -> WizardAction` — pure function; handles Enter/Backspace/Esc/Char correctly for all step types
- `WizardAction` enum: `Continue | Submit(WizardInputs) | Cancel`
- Dynamic step allocation: `fields` grows when chunk_count is confirmed at step 2 (3 + 2*N total steps)
- `draw_wizard(frame: &mut Frame, state: &WizardState)` free function — popup via `Clear` + `Flex::Center`, slug preview hint, inline error display, cursor positioned via `frame.set_cursor_position`
- `n` keybinding in Dashboard transitions to `Screen::Wizard(WizardState::new())`; Esc/Cancel returns to Dashboard
- On `Submit`: calls `create_from_inputs`, on success reloads milestones via `milestone_scan` and returns to Dashboard; on error stays in wizard with `state.error = Some(message)`
- Integration test `wizard_round_trip` in `crates/assay-tui/tests/wizard_round_trip.rs` — drives `handle_wizard_event` with synthetic `KeyEvent`s → `Submit(WizardInputs)` → `create_from_inputs` → asserts milestone TOML + chunk `gates.toml` files exist in tempdir (no terminal required)
- `crates/assay-tui/src/lib.rs` library target added so integration tests can import wizard types
- `tempfile.workspace = true` added to `[dev-dependencies]` in `crates/assay-tui/Cargo.toml`
- `cargo test -p assay-tui` passes; `cargo build -p assay-tui` succeeds

## Proof Level

- This slice proves: integration (wizard state machine + create_from_inputs file output verified by test) + contract (WizardState/WizardAction API)
- Real runtime required: no (integration test exercises the full TUI→filesystem round-trip without a terminal)
- Human/UAT required: yes — interactive keyboard navigation and popup rendering require visual verification

## Verification

Primary proof (automated):
- `cargo test -p assay-tui wizard_round_trip` — integration test drives synthetic KeyEvents through all steps for N=2 chunks, calls `create_from_inputs`, asserts milestone TOML and two `gates.toml` files exist in tempdir
- `cargo test -p assay-tui wizard_step` — unit tests for `current_step_kind()` at N in {1, 2, 3}: verifies step count = 3 + 2*N and step routing at each index
- `cargo test -p assay-tui wizard_backspace` — unit test: backspace on empty single-line step decrements `step`; backspace on empty criteria step when list is empty decrements `step`; backspace on non-empty step removes last char
- `cargo test -p assay-tui wizard_error` — unit test: feeding a duplicate slug causes `create_from_inputs` to return Err; wizard stays at current step with `state.error = Some(...)`, not at `Screen::Dashboard`
- `cargo test -p assay-tui` passes; `just ready` passes

Operational smoke (manual UAT):
- Launch `assay-tui` on a project with `.assay/`; press `n` → popup opens; fill all steps; press Enter on blank criteria to finish; new milestone appears in dashboard list

## Observability / Diagnostics

- Runtime signals: `state.error: Option<String>` visible inline on the active step during wizard; cleared on next valid keypress
- Inspection surfaces: `cargo test -p assay-tui wizard_round_trip -- --nocapture` shows step-by-step trace if test adds `eprintln!`; milestone TOML file at `.assay/milestones/<slug>.toml` after successful submission
- Failure visibility: on `create_from_inputs` failure (slug collision, I/O), error message is held in `state.error` and displayed in the wizard popup; wizard does NOT transition to dashboard
- Redaction constraints: none — no secrets or PII in wizard fields

## Integration Closure

- Upstream surfaces consumed:
  - `assay_core::wizard::{WizardInputs, WizardChunkInput, create_from_inputs, slugify}` (existing, tested)
  - `assay_core::milestone::milestone_scan` (existing, from S01)
  - `App { screen, milestones, list_state, project_root }` + `Screen::Wizard(WizardState)` variant (from S01 boundary map)
  - `ratatui::{widgets::Clear, layout::{Flex, Layout, Constraint}}`, `crossterm::event::{KeyCode, KeyEvent, KeyEventKind}` (existing workspace deps)
- New wiring introduced in this slice:
  - `App.handle_event`: `n` → `Screen::Wizard(WizardState::new())` in Dashboard match arm
  - `App.handle_event`: Screen::Wizard arm calls `handle_wizard_event` → matches `WizardAction`
  - `App.draw`: Screen::Wizard arm calls `draw_wizard(frame, state)` after dashboard render
- What remains before the milestone is truly usable end-to-end: S03 (chunk/spec browser), S04 (provider settings), S05 (help overlay, status bar, polish)

## Tasks

- [ ] **T01: Add library target and integration test contract** `est:45m`
  - Why: Establishes the test-first contract for `WizardState`/`WizardAction`; restructures `assay-tui` from pure binary to binary+library so integration tests can import types; adds `tempfile` dev-dep
  - Files: `crates/assay-tui/Cargo.toml`, `crates/assay-tui/src/lib.rs`, `crates/assay-tui/src/wizard.rs`, `crates/assay-tui/tests/wizard_round_trip.rs`
  - Do: (1) Add `[lib] name = "assay_tui" path = "src/lib.rs"` section + `tempfile.workspace = true` to `[dev-dependencies]` in Cargo.toml; (2) Create `src/lib.rs` with `pub mod wizard;`; (3) Create `src/wizard.rs` with stub types — `WizardState` with public fields matching the contract, `WizardAction` enum, `StepKind` enum, `impl WizardState { pub fn new() -> Self { ... } }` (unimplemented bodies ok); (4) Write `tests/wizard_round_trip.rs` — full integration test that constructs `WizardState::new()`, drives `handle_wizard_event` in a loop with synthetic `KeyEvent`s for N=2 chunks (name "Auth Layer", chunks "login"/"register", 1 criterion each), waits for `WizardAction::Submit(inputs)`, calls `create_from_inputs` on a `TempDir`, asserts milestone TOML and two `gates.toml` files exist; (5) Run `cargo build -p assay-tui` → succeeds (stubs compile); run `cargo test -p assay-tui wizard_round_trip` → test fails/panics (red state, expected)
  - Verify: `cargo build -p assay-tui` exits 0; `cargo test -p assay-tui wizard_round_trip 2>&1 | grep -E "FAILED|panicked|error\[E"` shows failure (not compile error — it must compile)
  - Done when: `assay-tui` compiles with a library + binary target; integration test compiles and runs but fails (red state, not a compile error); `cargo build -p assay-tui` still produces `target/debug/assay-tui` binary

- [ ] **T02: Implement WizardState state machine and make integration test green** `est:90m`
  - Why: Implements the full pure event-handling logic — step routing, Enter/Backspace/Esc/Char semantics, dynamic field allocation on chunk_count confirmation, criteria multi-line accumulation, Submit assembly; makes the integration test pass without a terminal
  - Files: `crates/assay-tui/src/wizard.rs`
  - Do: (1) Implement `StepKind` enum (`Name | Description | ChunkCount | ChunkName(usize) | Criteria(usize)`) and `WizardState::current_step_kind(&self) -> StepKind` using stored `chunk_count`; (2) Implement `WizardState::new()` — `step: 0`, `fields: vec![vec![String::new()], vec![String::new()], vec![String::new()]]`, `cursor: 0`, `chunk_count: 0`, `error: None`; (3) Implement full `handle_wizard_event` — guard `if key.kind != KeyEventKind::Press { return WizardAction::Continue; }`; for each step kind: `Char(c)` appends to active buffer and bumps `cursor`; `Backspace` on non-empty deletes last char; `Backspace` on empty single-line step decrements `step` (min 0); `Backspace` on Criteria when list is empty decrements step; Enter on Name/Description/ChunkName advances step; Enter on ChunkCount validates digit 1–7, sets `chunk_count`, allocates `chunk_count` ChunkName step fields + `chunk_count` Criteria step fields, advances step; Enter on Criteria with non-blank appends criterion and resets buffer; Enter on Criteria with blank advances to next step (or assembles Submit on last criteria step); Esc returns Cancel; (4) `handle_wizard_event` assembles `WizardInputs` on Submit from `fields` — milestone slug = `slugify(fields[0][0])`, name = `fields[0][0]`, description from fields[1][0] if non-empty, chunks from fields[3..3+N] (name) + fields[3+N..3+2N] (criteria); (5) Add unit tests in `src/wizard.rs` behind `#[cfg(test)]`: `wizard_step_kind_n1`, `wizard_step_kind_n2`, `wizard_step_kind_n3` (verify step count and routing), `wizard_backspace_on_empty_goes_back`, `wizard_criteria_blank_enter_advances`, `wizard_submit_assembles_inputs`; (6) Run `cargo test -p assay-tui` → all tests pass including `wizard_round_trip`
  - Verify: `cargo test -p assay-tui` exits 0; `cargo test -p assay-tui wizard_round_trip -- --nocapture` shows milestone file path; `cargo test -p assay-tui wizard_step` passes; `cargo test -p assay-tui wizard_backspace` passes
  - Done when: `cargo test -p assay-tui` passes (all unit + integration tests); `handle_wizard_event` is a pure function — no `ratatui::init()` or terminal touch anywhere in `wizard.rs`

- [ ] **T03: Implement draw_wizard and wire into App** `est:90m`
  - Why: Completes the slice by (a) rendering the wizard popup visually and (b) wiring the keybinding + action handling into the App so the full user flow works: Dashboard → `n` → Wizard → complete → refreshed Dashboard or → Esc → Dashboard; read S01-SUMMARY.md first for actual App/Screen shapes before editing main.rs
  - Files: `crates/assay-tui/src/wizard_draw.rs` (new), `crates/assay-tui/src/lib.rs`, `crates/assay-tui/src/main.rs`
  - Do: (1) Read S01-SUMMARY.md for actual App struct fields and Screen enum variants as produced by S01; (2) Create `src/wizard_draw.rs` and add `pub mod wizard_draw;` to `lib.rs`; implement `pub fn draw_wizard(frame: &mut Frame, state: &WizardState)` — centered popup: `Layout::vertical([Constraint::Fill(1), Constraint::Length(14), Constraint::Fill(1)]).flex(Flex::Center)` + horizontal equivalent to get a 62×14 block; render `Clear` first then a bordered `Block` with title "New Milestone"; inside: step counter line "Step N of M", prompt line for current step kind, active input buffer, dim slug hint (only when buffer non-empty, shows `slugify(buffer)` via `Span::styled("→ slug", gray)`), error line in red if `state.error.is_some()`, key hints line `[Enter] confirm  [Esc] cancel  [Backspace] back`; for Criteria steps show accumulated criteria above the active input line; guard popup width/height with `area.width.min(62)` etc.; call `frame.set_cursor_position((col, row))` at end to show hardware cursor in active field; (3) Wire into `main.rs` App: in `draw(frame)` match arm for `Screen::Wizard(state)` call `draw_wizard(frame, state)` after rendering the Dashboard behind it (so the popup overlays the dashboard); in `handle_event` match arm for `Screen::Wizard(state)`: call `handle_wizard_event(state, key)`, match on `WizardAction::Continue` (no-op), `WizardAction::Cancel` (set `self.screen = Screen::Dashboard`), `WizardAction::Submit(inputs)` (call `create_from_inputs(&inputs, assay_dir, specs_dir)`, on Err set `state.error = Some(e.to_string())`, on Ok call `milestone_scan(assay_dir)` to reload `self.milestones`, set `self.screen = Screen::Dashboard`); in Dashboard `handle_event` add `KeyCode::Char('n') if project_root.is_some()` → `self.screen = Screen::Wizard(WizardState::new())`; (4) Run `cargo test -p assay-tui` → all tests still pass; `cargo build -p assay-tui` → binary exists
  - Verify: `cargo test -p assay-tui` exits 0; `cargo build -p assay-tui` exits 0 producing `target/debug/assay-tui`; `just ready` exits 0 (fmt + lint + test + deny all pass)
  - Done when: `cargo test -p assay-tui` passes; `just ready` passes; `assay-tui` binary launches on a real `.assay/` project, `n` opens the wizard popup, completing the wizard shows the new milestone in the dashboard list, `Esc` returns to dashboard without creating files

## Files Likely Touched

- `crates/assay-tui/Cargo.toml` — add `[lib]`, add `tempfile.workspace = true` to dev-deps
- `crates/assay-tui/src/lib.rs` — new (T01: `pub mod wizard;`); updated (T03: adds `pub mod wizard_draw;`)
- `crates/assay-tui/src/wizard.rs` — new; WizardState, StepKind, WizardAction, handle_wizard_event, unit tests
- `crates/assay-tui/src/wizard_draw.rs` — new; draw_wizard free function
- `crates/assay-tui/src/main.rs` — wire n keybinding, Wizard screen draw/event dispatch, create_from_inputs call
- `crates/assay-tui/tests/wizard_round_trip.rs` — new; integration test
