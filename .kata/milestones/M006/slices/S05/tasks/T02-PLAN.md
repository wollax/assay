---
estimated_steps: 8
estimated_files: 5
---

# T02: Global layout split, status bar, help overlay, `?` key, resize fix, final just ready

**Slice:** S05 — Help Overlay, Status Bar, and Integration Polish
**Milestone:** M006

## Description

Implements all remaining S05 deliverables: the global layout split in `draw()` that threads `content_area` to all screen renderers and `status_area` to the new status bar; `draw_status_bar` and `draw_help_overlay` free functions; the `?` key handler and help-overlay event guard; `Event::Resize` handling in `run()`; and cycle_slug refresh in the Settings save path. Concludes with `just ready` final pass. Turns all 6 `help_status` tests green.

## Steps

1. **Update all `draw_*` function signatures to accept `area: Rect`** (prerequisite for the layout split). In `crates/assay-tui/src/app.rs`, change each function signature: `draw_no_project(frame)` → `draw_no_project(frame, area: Rect)`; `draw_load_error(frame, msg)` → `draw_load_error(frame, area: Rect, msg: &str)`; `draw_dashboard(frame, milestones, list_state)` → `draw_dashboard(frame, area: Rect, milestones: &[Milestone], list_state: &mut ListState)`; `draw_milestone_detail(frame, milestone, list_state)` → `draw_milestone_detail(frame, area: Rect, milestone: Option<&Milestone>, list_state: &mut ListState)`; `draw_chunk_detail(frame, chunk_slug, spec, note, run)` → `draw_chunk_detail(frame, area: Rect, chunk_slug: &str, spec: Option<&GatesSpec>, note: Option<&str>, run: Option<&GateRunRecord>)`. In each function body, replace `let area = frame.area();` with the passed `area` parameter. In `crates/assay-tui/src/wizard.rs`, change `draw_wizard(frame, state)` → `draw_wizard(frame: &mut Frame, area: Rect, state: &WizardState)` and update the internal base-rect computation to use `area` instead of `frame.area()`. In S04's `crates/assay-tui/src/settings.rs`, change `draw_settings(frame, ...)` to add `area: Rect` as the second parameter, replacing any internal `frame.area()` call.

2. **Implement the global layout split in `draw()`**. Replace the body of `App::draw` with: `let [content_area, status_area] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());`. Then update every screen match arm call to pass `content_area` as the first positional argument after `frame`: `draw_no_project(frame, content_area)`, `draw_dashboard(frame, content_area, ...)`, `draw_milestone_detail(frame, content_area, ...)`, `draw_chunk_detail(frame, content_area, ...)`, `draw_load_error(frame, content_area, ...)`, `draw_wizard(frame, content_area, ...)`, `draw_settings(frame, content_area, ...)` (S04's variant). Add `use ratatui::layout::Rect;` to the import block if not already present.

3. **Implement `draw_status_bar`** as a free function in `app.rs`: `fn draw_status_bar(frame: &mut ratatui::Frame, area: Rect, project_name: &str, cycle_slug: Option<&str>)`. Build the line as: project name (left), separator `"  ·  "`, cycle slug or blank (dim), separator `"  ·  "` (dim), `"? help  q quit"` (dim). Use `ratatui::text::Span` to compose the line. Render as a single `Paragraph`. Call it from `draw()` after the match: `let project_name = self.config.as_ref().map(|c| c.project_name.as_str()).unwrap_or(""); draw_status_bar(frame, status_area, project_name, self.cycle_slug.as_deref());`.

4. **Implement `draw_help_overlay`** as a free function in `app.rs`: `fn draw_help_overlay(frame: &mut ratatui::Frame, area: Rect)`. Compute centered popup rect: `let w = area.width.min(62); let h = 22; let x = area.x + (area.width.saturating_sub(w)) / 2; let y = area.y + (area.height.saturating_sub(h)) / 2; let popup = Rect::new(x, y, w, h);`. Render `Clear` at `popup`. Render `Block::bordered().title(" Keybindings — press ? or Esc to close ")` at `popup`. Compute inner area via `block.inner(popup)`. Build a `Table` with two-column `Row`s grouped as: Global (`?` toggle help, `q` quit), Dashboard (`↑↓` navigate, `Enter` open milestone, `n` new, `s` settings), Detail views (`Enter` open chunk, `Esc` back to parent), Wizard (`Enter` next step / confirm, `Backspace` delete / prev step, `Esc` cancel), Settings (`↑↓` select provider, `w` save, `Esc` / `q` cancel). Add `use ratatui::widgets::Clear;` to imports. Call `draw_help_overlay(frame, frame.area())` at the very end of `draw()`, after all other renders, when `self.show_help` is true.

5. **Wire the `?` key handler and help-overlay event guard** in `App::handle_event`. Add at the very beginning of the method body, before the `match self.screen` block: `// When help overlay is visible, only ? and Esc dismiss it; all other keys are no-ops. if self.show_help { if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) { self.show_help = false; } return false; }`. Then add: `// Global ? key opens help from any non-wizard screen. if key.code == KeyCode::Char('?') && !matches!(self.screen, Screen::Wizard(_)) { self.show_help = true; return false; }`.

6. **Add `cycle_slug` refresh in the Settings save path**. After S04's `w` key success path transitions back to `Screen::Dashboard`, add: `if let Some(root) = &self.project_root { let ad = root.join(".assay"); self.cycle_slug = cycle_status(&ad).ok().flatten().map(|cs| cs.milestone_slug); }`. The exact location is in the `Screen::Settings` arm, in the `w` key handler, after `config_save` succeeds and before or after `self.screen = Screen::Dashboard;`.

7. **Fix `Event::Resize` handling in `main.rs`**. Replace `if let Event::Key(key) = event::read()? && app.handle_event(key) { break; }` with: `match event::read()? { crossterm::event::Event::Key(key) => { if app.handle_event(key) { break; } }, crossterm::event::Event::Resize(..) => { terminal.clear()?; }, _ => {} }`. Remove the `if let` import form (use `crossterm::event::Event` fully qualified or add `use crossterm::event::Event;`).

8. **Run `just ready` and fix any issues**. The likely issues: `draw_wizard` in `wizard.rs` needs `Rect` in scope (add `use ratatui::layout::Rect;`); lint warnings from unused imports after refactor; potential `fmt` changes. Fix all to achieve green. Run `cargo test -p assay-tui --test help_status` and confirm all 6 pass. Run `cargo test --workspace` and confirm all tests pass. Then run `just ready` for the final milestone gate.

## Must-Haves

- [ ] All `draw_*` functions accept `area: Rect` as explicit parameter (no internal `frame.area()` calls)
- [ ] `draw()` global layout split produces `[content_area, status_area]`; all screen renderers receive `content_area`
- [ ] `draw_status_bar` renders project name + cycle slug + dim hints in `status_area`
- [ ] `draw_help_overlay` renders centered bordered table of all keybindings with `Clear` backing
- [ ] Help overlay rendered last in `draw()` when `self.show_help == true` (on top of all other content)
- [ ] `?` key: sets `show_help = true` from any screen except `Screen::Wizard`
- [ ] When `show_help == true`: only `?` or `Esc` dismiss it; all other keys are no-ops (no accidental navigation)
- [ ] `cycle_slug` refreshed after Settings `w` save
- [ ] `Event::Resize` in `run()` calls `terminal.clear()` instead of being silently dropped
- [ ] All 6 `help_status` tests pass
- [ ] All prior assay-tui tests pass (no regressions from signature refactor)
- [ ] `just ready` — "All checks passed"

## Verification

- `cargo test -p assay-tui --test help_status` — 6/6 pass
- `cargo test -p assay-tui` — all prior tests pass (≥ 16 total from S01–S04)
- `cargo test --workspace` — ≥ 1371 tests pass
- `just ready` — fmt ✓, lint ✓, test ✓, deny ✓ — "All checks passed"
- `grep "frame.area()" crates/assay-tui/src/app.rs` — zero matches (all area calls removed from draw_* bodies)

## Observability Impact

- Signals added/changed: `draw_status_bar` is the persistent runtime signal showing project context; `draw_help_overlay` makes all keybindings discoverable without docs
- How a future agent inspects this: `app.show_help` field — check it when diagnosing unexpected keypress behavior; `app.cycle_slug` — check when status bar shows no milestone slug
- Failure state exposed: resize events no longer silently drop (Resize → terminal.clear() → clean redraw on next frame)

## Inputs

- `crates/assay-tui/src/app.rs` — all existing draw_* functions to refactor + App::draw + App::handle_event to extend
- `crates/assay-tui/src/wizard.rs` — `draw_wizard` signature to update
- `crates/assay-tui/src/settings.rs` — `draw_settings` signature to update (added by S04)
- `crates/assay-tui/src/main.rs` — `run()` event loop to fix for `Event::Resize`
- `crates/assay-tui/tests/help_status.rs` — T01's test file; T02 makes all 6 tests green

## Expected Output

- `crates/assay-tui/src/app.rs` — global layout split in `draw()`; all `draw_*` signatures accept `area: Rect`; `draw_status_bar` and `draw_help_overlay` free functions; `?` handler + help-overlay guard in `handle_event`; cycle_slug refresh in Settings `w` path
- `crates/assay-tui/src/wizard.rs` — `draw_wizard` signature updated to `(frame, area: Rect, state)`
- `crates/assay-tui/src/settings.rs` — `draw_settings` signature updated to include `area: Rect`
- `crates/assay-tui/src/main.rs` — `run()` uses `match event::read()?` handling both `Key` and `Resize` variants
- `just ready` → "All checks passed" — M006 milestone complete
