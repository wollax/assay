---
estimated_steps: 6
estimated_files: 4
---

# T03: Implement draw_wizard and wire into App

**Slice:** S02 — In-TUI Authoring Wizard
**Milestone:** M006

## Description

Completes the slice by adding the visual layer (`draw_wizard`) and wiring the wizard into `App`. After this task: pressing `n` from the Dashboard opens the wizard popup; the user fills the form; on completion `create_from_inputs` is called and the Dashboard reloads showing the new milestone; on Esc the wizard closes with no side effects.

**Critical prerequisite:** Read `S01-SUMMARY.md` before editing `main.rs`. S01 defines the actual `App` struct fields and `Screen` enum variants — this task must match those shapes exactly. The boundary map describes the expected shapes, but S01 may have made minor adjustments; the summary is authoritative.

This task does NOT require new tests — `draw_wizard` is a rendering function verified by visual UAT, and the App wiring is covered by the existing integration test (which exercises the Submit path end-to-end). The focus is on getting `just ready` green with the full wiring in place.

## Steps

1. Read `S01-SUMMARY.md` at `.kata/milestones/M006/slices/S01/S01-SUMMARY.md` — note actual `App` struct fields, `Screen` enum variants, how `handle_event` dispatches, how `draw` dispatches. If the shapes differ from the boundary map (e.g. `screen` is stored differently, `project_root` has a different name), adapt accordingly.

2. Create `crates/assay-tui/src/wizard_draw.rs` with `draw_wizard(frame: &mut Frame, state: &WizardState)`:
   - Centered popup geometry: outer vertical split `[Fill(1), Length(14), Fill(1)]` with `.flex(Flex::Center)`, inner horizontal split `[Fill(1), Length(64), Fill(1)]` with `.flex(Flex::Center)` on the middle row — gives a 64×14 popup area
   - Guard: `let popup_area = Rect { width: area.width.min(64), height: area.height.min(14), ..area }` to prevent Layout panics on narrow terminals
   - Render `Clear` widget first: `frame.render_widget(Clear, popup_area)` — clears dashboard text behind popup
   - Render bordered `Block` with title `" New Milestone "` and inner area via `block.inner(popup_area)`
   - Inside inner area, build a `Vec<Line>` of content lines:
     - Line 1: `"Step N of M"` (dim style) where N = state.step + 1, M = total_steps (from chunk_count or estimated 5 if not yet known)
     - Line 2: prompt string for current step kind (e.g. "Milestone name:", "Description (optional):", "Number of chunks (1–7):", "Chunk N name:", "Chunk N criteria — one per line, blank line to finish:")
     - Lines 3+: for Criteria step — render each accumulated criterion as a dim line with a `•` prefix
     - Active input line: `"> " + active_buffer` (active buffer is `state.fields[state.step].last().unwrap_or(&String::new())`)
     - Slug hint line: if buffer non-empty and step is Name/ChunkName, show `Span::styled(format!("  → {}", slugify(buffer)), Style::default().dim())` on next line
     - Error line: if `state.error.is_some()`, render it in red: `Span::styled(error, Style::default().fg(Color::Red))`
     - Bottom key-hint line: `"[Enter] confirm  [Esc] cancel  [Backspace] back"` (dim)
   - Render `Paragraph::new(lines)` into inner area
   - Compute cursor position: row = top of inner area + (2 + accumulated_criteria_count); col = inner_area.x + 2 ("> ") + `state.cursor as u16`; call `frame.set_cursor_position((col, row))`

3. Wire wizard rendering into `main.rs` App `draw` method:
   - In the `Screen::Wizard(state)` match arm: first call the Dashboard render (so the dashboard is visible behind the popup), then call `draw_wizard(frame, state)`
   - Add `use crate::wizard_draw::draw_wizard;` (or `use assay_tui::wizard_draw::draw_wizard;` if in binary context)

4. Wire wizard event handling into `main.rs` App `handle_event` method:
   - Add `Screen::Wizard(state)` arm: call `handle_wizard_event(state, key)` on any key event; match result:
     - `WizardAction::Continue` → no-op
     - `WizardAction::Cancel` → `self.screen = Screen::Dashboard`
     - `WizardAction::Submit(inputs)` → call `create_from_inputs(&inputs, assay_dir, specs_dir)` where `assay_dir = project_root.join(".assay")` and `specs_dir = assay_dir.join("specs")`; on `Err(e)` → set `state.error = Some(e.to_string())` (stays in wizard); on `Ok(_)` → reload `self.milestones = milestone_scan(&assay_dir).unwrap_or_default()`, reset `self.list_state` selection to 0, set `self.screen = Screen::Dashboard`
   - In the `Screen::Dashboard` match arm, add: `KeyCode::Char('n') if self.project_root.is_some()` → `self.screen = Screen::Wizard(WizardState::new())`

5. Run `cargo test -p assay-tui` → all tests still pass; `cargo build -p assay-tui` → exits 0

6. Run `just ready` → fmt + lint + test + deny all pass; fix any clippy warnings (common: unused imports, dead_code on wizard_draw functions before they're called in tests)

## Must-Haves

- [ ] `draw_wizard` renders a popup via `Clear` + centered `Block` — no bleed-through of dashboard text
- [ ] `draw_wizard` shows current step prompt, active input buffer, slug hint (when applicable), accumulated criteria (for criteria steps), and inline error (when `state.error.is_some()`)
- [ ] `frame.set_cursor_position` called at end of `draw_wizard` with correct (col, row)
- [ ] Popup area guarded against narrow terminals: `width.min(64)`, `height.min(14)`
- [ ] `n` in Dashboard → `Screen::Wizard(WizardState::new())` (only when `project_root.is_some()`)
- [ ] `WizardAction::Cancel` → `Screen::Dashboard` (no files written)
- [ ] `WizardAction::Submit` → `create_from_inputs` called; on success milestones reloaded + screen = Dashboard; on error `state.error` set and screen stays Wizard
- [ ] `cargo test -p assay-tui` passes
- [ ] `just ready` passes (fmt + lint + test + deny)

## Verification

- `cargo test -p assay-tui` exits 0 — all tests pass (integration test was already green from T02)
- `cargo build -p assay-tui` exits 0 — `target/debug/assay-tui` exists
- `just ready` exits 0 — fmt, lint, test, deny all clean
- Manual UAT (on a real `.assay/` project): launch `assay-tui`; press `n` → popup appears; fill name, description, chunk count 2, two chunk names, one criterion per chunk; last blank Enter → wizard closes; new milestone visible in dashboard list; press `Esc` mid-wizard → returns to dashboard with no file written

## Observability Impact

- Signals added/changed: `state.error` rendered inline on the wizard popup — user sees I/O errors immediately; `App.milestones` reloaded after success — dashboard reflects new state without restart
- How a future agent inspects this: `cargo test -p assay-tui wizard_round_trip` exercises the Submit path; manual UAT exercises the visual path; `ls .assay/milestones/` after wizard completion confirms file was written
- Failure state exposed: `create_from_inputs` errors (slug collision, I/O) surface in the wizard UI as red error text; `milestone_scan` failure after success silently uses empty list (non-fatal — user can restart)

## Inputs

- `S01-SUMMARY.md` — **read first**; authoritative `App` struct and `Screen` enum shapes from S01
- `crates/assay-tui/src/wizard.rs` — `WizardState`, `WizardAction`, `handle_wizard_event`, `StepKind` (from T02)
- `crates/assay-tui/src/main.rs` — S01 App implementation to wire into
- `crates/assay-core/src/wizard.rs` — `create_from_inputs`, `WizardInputs` signatures
- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan` signature
- `~/.cargo/registry/.../ratatui-core-0.1.0/src/terminal/frame.rs` — `set_cursor_position` API
- `~/.cargo/registry/.../ratatui-core-0.1.0/src/layout/flex.rs` — `Flex::Center` for centered popup layout
- `~/.cargo/registry/.../ratatui-widgets-0.3.0/src/clear.rs` — `Clear` widget for popup overdraw

## Expected Output

- `crates/assay-tui/src/wizard_draw.rs` — new; `draw_wizard` free function
- `crates/assay-tui/src/lib.rs` — updated; `pub mod wizard_draw;` added
- `crates/assay-tui/src/main.rs` — updated; `n` keybinding, Wizard screen dispatch, create_from_inputs + reload wiring
- `just ready` green; `assay-tui` binary launches and wizard works end-to-end
