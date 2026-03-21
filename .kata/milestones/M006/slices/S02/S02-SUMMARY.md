---
id: S02
parent: M006
milestone: M006
provides:
  - WizardState struct with step, fields: Vec<Vec<String>>, cursor, chunk_count, error: Option<String>
  - StepKind enum (Name | Description | ChunkCount | ChunkName(usize) | Criteria(usize)) with current_step_kind() helper
  - handle_wizard_event(state, key) — pure state machine for all wizard step types and key codes
  - WizardAction enum (Continue | Submit(WizardInputs) | Cancel) — clean return type from pure function
  - Dynamic field allocation — chunk_count confirmation pushes N ChunkName + N Criteria vecs
  - draw_wizard(frame, state) — centered 64×14 popup: step counter, prompt, accumulated criteria, input buffer, slug hint, inline error, key hints, hardware cursor via set_cursor_position
  - App wiring: n → Screen::Wizard(WizardState::new()), Submit → create_from_inputs + milestone_scan reload, Esc → Dashboard
  - Integration test wizard_round_trip: drives synthetic KeyEvents through all steps, asserts milestone TOML + gates.toml files written to tempdir
  - 23 assay-tui tests total (14 unit + 8 app + 1 integration) — all green
requires:
  - slice: S01
    provides: App struct (screen/milestones/list_state/project_root), Screen enum with Wizard variant, run/draw/handle_event foundation
affects:
  - S05
key_files:
  - crates/assay-tui/src/wizard.rs
  - crates/assay-tui/src/wizard_draw.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/Cargo.toml
  - crates/assay-tui/tests/wizard_round_trip.rs
key_decisions:
  - WizardState lives in assay-tui, not assay-core — wizard state is a TUI concern; core provides create_from_inputs only
  - ChunkCount accepts only '1'–'7' via replace-semantics (not append); invalid chars silently ignored, not surfaced as errors
  - assemble_submit() is a free function to avoid borrow conflicts when reading WizardState after mutable event handling
  - Combined bin+lib crate pattern: app.rs in binary tree (mod app in main.rs), imports wizard types via `assay_tui::` lib path
  - Popup geometry uses manual centered Rect math (not Layout::flex) — simpler for fixed 64×14 dimensions
  - draw() always renders Dashboard first, then overlays wizard popup if Screen::Wizard — avoids borrow split
patterns_established:
  - Pure event function pattern: handle_wizard_event takes &mut WizardState + KeyEvent, returns owned WizardAction — no terminal, no side effects
  - Popup overlay pattern: render Clear first, then Block, then content in block.inner(); set_cursor_position at end
  - Error-in-state pattern: state.error: Option<String> set on failure, cleared on next keypress, rendered inline in popup as red text
observability_surfaces:
  - state.error: Option<String> — inline error visible in wizard popup for create_from_inputs failures (slug collision, I/O)
  - cargo test -p assay-tui wizard_round_trip -- --nocapture — exercises full TUI→filesystem round-trip without a terminal
  - .assay/milestones/<slug>.toml written on successful Submit — inspectable on disk immediately
drill_down_paths:
  - .kata/milestones/M006/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M006/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M006/slices/S02/tasks/T03-SUMMARY.md
duration: ~3.5h
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
---

# S02: In-TUI Authoring Wizard

**Multi-step Ratatui authoring wizard fully implemented — pure WizardState state machine, draw_wizard popup, App wiring, and wizard_round_trip integration test all green; 23 assay-tui tests + 1356 workspace tests pass.**

## What Happened

Three tasks across the highest-risk slice in M006 — the wizard form state machine.

**T01** restructured `assay-tui` from a pure binary into a binary+library crate. Added `[lib]` section in Cargo.toml, created `src/lib.rs` with `pub mod wizard;`, created `src/wizard.rs` with stub types (all `todo!("T02")`), and wrote the full `tests/wizard_round_trip.rs` integration test. The test compiled and panicked at the stubs — confirmed red state, no compile errors.

**T02** replaced all stubs with the complete pure state machine. `current_step_kind()` maps raw step indices to `StepKind` variants using stored `chunk_count`. `handle_wizard_event()` guards on `KeyEventKind::Press`, clears `state.error` on every press, then dispatches: Char input with ChunkCount replace-semantics; Backspace pop-or-go-back per step type; Enter with validation and dynamic field allocation at ChunkCount; multi-entry criteria with blank-Enter advance/submit; Esc cancels immediately. `assemble_submit()` builds `WizardInputs` with `slugify()` and filters empty criterion strings. 13 unit tests + wizard_round_trip integration test all passed.

**T03** merged origin/main (S01's `app.rs`) to get the App/Screen foundation, created `wizard_draw.rs` with the popup renderer, and wired everything into App. The S01 branch had landed to main before T03 began — merge resolved Cargo.toml conflict (combined `[lib]` + `[[bin]]`). `draw_wizard` builds a centered 64×14 Rect manually (no Layout::flex), renders Clear → Block → content lines including accumulated criteria, slug hint, inline error, key hints. App: `n` keybinding opens wizard, Submit path calls `create_from_inputs`, reloads milestones via `milestone_scan`, returns to Dashboard; errors set `state.error` and keep wizard open. 3 new App-level tests added.

## Verification

```
cargo test -p assay-tui                          # 23 tests — all pass (14 unit + 8 app + 1 integration)
cargo build -p assay-tui                         # exits 0; target/debug/assay-tui produced
cargo clippy --workspace --all-targets -- -D warnings   # clean
cargo fmt --all -- --check                       # clean
cargo test --workspace                           # 1356 tests total — all pass
```

Specific named checks from slice plan:
- `cargo test -p assay-tui wizard_round_trip` — passes; exercises Submit path end-to-end without a terminal
- `cargo test -p assay-tui wizard_step` — passes (step routing for N in {1,2,3})
- `cargo test -p assay-tui wizard_backspace` — passes (backspace nav and pop)
- `cargo test -p assay-tui wizard_error` — passes (error stays in wizard, not dashboard)

Note: `just ready` deny check fails on 6 pre-existing `aws-lc-sys` CVEs (RUSTSEC-2026-0044 to -0049) via `jsonschema` dev-dep in `assay-types`. These were present before S02 began and are not introduced by this slice.

## Requirements Advanced

- R050 (TUI interactive wizard) — now validated: wizard runs inside TUI as interactive form; creating milestone without dropping to CLI proven by integration test and app wiring

## Requirements Validated

- R050 — wizard_round_trip integration test drives synthetic KeyEvents through all steps → create_from_inputs → asserts milestone TOML + chunk gates.toml written to tempdir; App wiring proves n→wizard→Dashboard flow; inline error handling covers the failure path

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- S01's `app.rs` had to be merged in from origin/main at T03 start — S02 branch was created before S01's PR #158 merged. Resolved Cargo.toml conflict (combined `[lib]` + `[[bin]]`).
- Popup geometry uses manual centered Rect math instead of `Layout::flex(Flex::Center)` — simpler for a fixed-size popup; same visual result.
- `project_root` is `PathBuf` (not `Option<PathBuf>`) in S01's actual App; `n` keybinding guards on `matches!(app.screen, Screen::Dashboard)` — no `project_root.is_some()` check needed.

## Known Limitations

- `just ready` deny check fails on 6 pre-existing `aws-lc-sys` CVEs — not introduced by S02; tracked as a separate fix needed before M006 completion.
- Visual rendering of the wizard popup (terminal size, color output, cursor positioning) requires human UAT — not covered by automated tests.
- Criteria entered in the wizard are text-only descriptions (no `cmd` field), consistent with D076; generated gates.toml entries are not immediately runnable without manual editing.

## Follow-ups

- Pre-existing `aws-lc-sys` CVE deny failures need `cargo update -p aws-lc-sys` fix before `just ready` can fully pass — should be addressed in S05 polish or a dedicated fix task.
- S03 should read the actual `App`/`Screen` struct in `app.rs` before modifying — the real shapes differ slightly from the roadmap boundary map (e.g. `project_root: PathBuf` not `Option<PathBuf>`).

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added `[lib]` section and `tempfile` dev-dependency
- `crates/assay-tui/src/lib.rs` — new; `pub mod wizard; pub mod wizard_draw;`
- `crates/assay-tui/src/wizard.rs` — new; WizardState, StepKind, WizardAction, handle_wizard_event, assemble_submit, 13 unit tests
- `crates/assay-tui/src/wizard_draw.rs` — new; draw_wizard free function (centered popup, hardware cursor)
- `crates/assay-tui/src/app.rs` — Screen::Wizard variant, n keybinding, wizard draw/event dispatch, create_from_inputs + reload, 3 new App tests
- `crates/assay-tui/tests/wizard_round_trip.rs` — new; integration test driving synthetic KeyEvents → create_from_inputs → filesystem assertions

## Forward Intelligence

### What the next slice should know
- The actual `App` struct is in `app.rs` (not `main.rs`); `main.rs` has `mod app;` and calls `app::run()`. When S03 adds `Screen::MilestoneDetail` and `Screen::ChunkDetail`, edit `app.rs`.
- `project_root` on `App` is `PathBuf` (not `Option<PathBuf>`) — the `NoProject` guard happens in `App::new()` before the struct is fully initialized. Navigation event guards use `matches!(self.screen, Screen::Dashboard)` not `project_root.is_some()`.
- `Screen::Wizard(WizardState)` pattern means the variant owns the state inline. S03 variants `MilestoneDetail` and `ChunkDetail` should follow the same pattern for their list states.
- The wizard imports `assay_tui::wizard::*` in the binary via the lib crate — not directly via `mod wizard;`. S03 can add `pub mod chunk_detail;` to `lib.rs` for the same pattern.

### What's fragile
- `draw()` renders Dashboard unconditionally first, then overlays wizard popup if needed — this coupling means Dashboard is always rendered even when not visible. For S03/S04 full-screen detail views, the draw dispatch will need to use a proper match on screen variant instead.
- The manual centered Rect in `draw_wizard` hardcodes 64 wide × 14 tall — if terminal is smaller than 64 columns, width clamps but the content may still overflow. No explicit minimum terminal size check exists.

### Authoritative diagnostics
- `cargo test -p assay-tui -- --nocapture` — most useful diagnostic; all unit + integration tests with output
- `state.error: Option<String>` — only failure signal during wizard execution; inspect this field when Submit doesn't transition to Dashboard

### What assumptions changed
- S02 plan assumed `project_root: Option<PathBuf>` in App (from boundary map); S01 actually uses `PathBuf` with `Screen::NoProject` for the missing-dir case. The wizard wiring adjusted accordingly.
- S02 plan proposed `Layout::flex(Flex::Center)` for popup centering; manual Rect centering proved simpler and avoids needing the Flex import.
