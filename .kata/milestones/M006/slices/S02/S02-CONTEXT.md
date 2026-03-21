---
id: S02
milestone: M006
status: ready
---

# S02: In-TUI Authoring Wizard — Context

## Goal

Deliver a multi-step Ratatui form that collects milestone name, description, chunk count, per-chunk names, and per-chunk criteria, then calls `create_from_inputs` to write real TOML files and returns the user to a refreshed dashboard.

## Why this Slice

S01 establishes the `App` struct, `Screen` enum (including the `Wizard` variant), and dashboard. S02 activates that variant with a real form state machine — this is the primary creation path for users who don't want to hand-edit TOML files. It retires the wizard form complexity risk (the highest-rated risk in the milestone) and proves the round-trip from TUI input → assay-core → filesystem.

## Scope

### In Scope

- `WizardState` struct in `assay-tui`: `step`, `fields`, `cursor`, `chunk_count`, and any error state needed for inline error display
- Step sequence: milestone name → optional description → chunk count (numeric input) → for each chunk: chunk name → criteria (one per line, blank line to finish)
- Slug preview: dim hint line below each name field showing the derived slug as the user types
- Backspace-on-empty-field goes back to the previous step; Esc cancels the entire wizard (all progress discarded) and returns to dashboard
- Minimal validation: milestone name required, each chunk name required; blank criteria is allowed; inline error hint shown if user tries to advance with a blocked field
- On `create_from_inputs` failure (e.g. slug collision, I/O error): stay in wizard, show inline error message on the current step, let user correct and retry — chunk data is preserved
- On success: return to dashboard with the new milestone visible in the list (pre-selected); brief status bar message "Created milestone <slug>" persists until next keypress
- Integration test `wizard_round_trip`: fills all steps via synthetic `KeyEvent`s → `create_from_inputs` → asserts milestone TOML and chunk spec files written to tempdir
- `draw_wizard(frame, state)` free function and `handle_wizard_event(state, event) -> WizardAction` with `WizardAction { Continue | Submit(WizardInputs) | Cancel }`

### Out of Scope

- `tui-textarea` crate or any third-party text input widget — implement with a simple `Vec<String>` line buffer and crossterm key events
- Multi-cursor or rich text editing within criteria fields
- Editing or deleting an existing milestone from the wizard
- Wizard-level undo history beyond single-field backspace
- Description field validation (empty description is valid; blank advances silently)
- Slug customisation — slug is always auto-derived from the name; no manual override in the wizard
- Criteria `cmd` field collection — wizard produces criteria with name only; `cmd` requires manual TOML editing post-wizard (known limitation, D076)

## Constraints

- **D090**: `WizardState` lives in `assay-tui`, not `assay-core`. Core provides `create_from_inputs`; TUI provides all form rendering and event handling.
- **D001 / D089**: Free render functions only — `draw_wizard` is a free `fn`, not a `Widget` impl. No trait objects on wizard state.
- **D091**: No blocking I/O inside `terminal.draw()`. `create_from_inputs` is called in `handle_wizard_event` on the final Submit action, not during rendering.
- **No `dialoguer`**: Cannot call `dialoguer` from within the Ratatui event loop (Pitfall 2, research). All input collection uses crossterm `KeyEvent`s and the wizard's own field buffers.
- **S01 prerequisite**: `Screen::Wizard(WizardState)` variant must already exist in the `Screen` enum and `App.project_root` must be available before this slice begins. Do not start S02 work on an S01-incomplete branch.

## Integration Points

### Consumes

- `assay-core::wizard::{WizardInputs, WizardChunkInput, create_from_inputs}` — called on Submit to write milestone TOML and chunk spec files atomically
- `assay-core::wizard::slugify` — used to derive and preview slugs as the user types
- `App.project_root: Option<PathBuf>` (from S01) — passed as `assay_dir` to `create_from_inputs`; wizard is unreachable when `project_root` is None (no-project guard in S01)
- `Screen::Wizard(WizardState)` variant (from S01) — the existing variant slot the form state machine plugs into
- `assay-core::milestone::milestone_scan` — called after successful `create_from_inputs` to reload the dashboard milestone list before returning to `Screen::Dashboard`

### Produces

- `WizardState { step: usize, fields: Vec<Vec<String>>, cursor: usize, chunk_count: usize, error: Option<String> }` — multi-step form state; one `Vec<String>` per step for multi-line fields (criteria)
- `draw_wizard(frame: &mut Frame, state: &WizardState)` — free render function; shows current step prompt, active field buffer, slug preview hint, and any inline error
- `handle_wizard_event(state: &mut WizardState, event: KeyEvent) -> WizardAction` — pure event handler; returns `Continue`, `Submit(WizardInputs)`, or `Cancel`
- `WizardAction` enum: `Continue | Submit(WizardInputs) | Cancel`
- Integration test `wizard_round_trip` in `crates/assay-tui/tests/` — proves the full TUI-to-filesystem round-trip via synthetic key events

## Open Questions

- **Chunk count input UX**: Should the user type a number (e.g. "3") or use arrow keys to increment/decrement? Current thinking: type a digit (1–7) and Enter; non-digit input is ignored; the step shows "Chunks (1–7):" prompt with current value. Simple and consistent with the rest of the text-input model.
- **Criteria step rendering when chunk count > 3**: With many chunks, the wizard will show many sequential criteria steps. No pagination needed for S02 — the step counter ("Step 5 of 9") and chunk label ("Chunk 2 of 4: criteria") should be enough orientation. Revisit if UAT feedback shows confusion.
- **Status bar flash duration**: "Brief" is intentional — exact duration (keypress-to-clear vs timed fade) left to the S02 planner. Keypress-to-clear is simpler and consistent with S05's planned status bar architecture; timed fade requires a timer and is out of scope for S02.
