# S05: Help Overlay, Status Bar, and Integration Polish

**Goal:** Close M006 by adding a `?`-triggered help overlay listing all keybindings, a persistent bottom status bar showing project name and active milestone slug, terminal resize handling, and a final `just ready` pass — with all S01–S04 screens working end-to-end.

**Demo:** `assay-tui` launches and shows a one-line bottom status bar (project name · active-milestone slug · dim key hints). Pressing `?` from any screen overlays a centered popup table of all keybindings. Pressing `?` or `Esc` dismisses it. Resizing the terminal produces no artifacts. `just ready` passes (fmt, lint, test, deny). The full flow — dashboard → chunk detail → wizard → provider config — works on a real `.assay/` project.

## Must-Haves

- `App.show_help: bool` field; toggled by `?` key from any screen except the Wizard; when true, `Esc` closes it and all other keys are consumed without side effects (no accidental navigation through the help overlay)
- `App.cycle_slug: Option<String>` field loaded in `with_project_root` via `assay_core::milestone::cycle_status`; refreshed after wizard Submit success and after Settings `w` save
- `draw_status_bar(frame, area, project_name: &str, cycle_slug: Option<&str>)` free function rendering project name · cycle slug · dim key hints in a 1-line bottom bar
- `draw_help_overlay(frame, area)` free function rendering a centered `~60×20` popup with `Clear` widget + `Block` + two-column `Table` of all keybindings (key / action)
- Global layout split in `draw()`: `frame.area()` carved into `[content_area, status_area]` (status = `Constraint::Length(1)`) before any per-screen dispatch; `content_area` passed to all per-screen renderers; help overlay rendered last (on top of everything) when `self.show_help`
- All `draw_*` free functions updated to accept explicit `area: Rect` parameter (remove internal `frame.area()` calls): `draw_dashboard`, `draw_milestone_detail`, `draw_chunk_detail`, `draw_no_project`, `draw_load_error` in `app.rs`; `draw_wizard` in `wizard.rs`; `draw_settings` in `settings.rs` (added by S04)
- `Event::Resize` handled in `run()` in `main.rs`: match all `Event` variants; on `Event::Resize(..)` call `terminal.clear()?` and continue; no more silent resize-event drop
- `just ready` passes: fmt, lint, test, deny all green; no regressions

## Proof Level

- This slice proves: final-assembly
- Real runtime required: no (tests use `App::with_project_root` + tempdir fixtures; no real terminal needed)
- Human/UAT required: yes (visual verification of status bar appearance, help overlay layout, resize behavior, end-to-end flow)

## Verification

- `cargo test -p assay-tui --test help_status` — 6 tests in `tests/help_status.rs` all pass
- `cargo test -p assay-tui` — all prior tests still pass (no regressions from `draw_*` signature refactor)
- `cargo test --workspace` — ≥ 1371 workspace tests pass (number from STATE.md current baseline)
- `just ready` — fmt ✓, lint ✓, test ✓, deny ✓ — "All checks passed"

## Observability / Diagnostics

- Runtime signals: `App.show_help` field — `true` means overlay is visible; `false` means hidden; directly testable in unit tests
- Inspection surfaces: `App.cycle_slug` field — `Some(slug)` when an `InProgress` milestone exists; `None` otherwise; `cycle_status` I/O errors degrade gracefully to `None` (status bar renders without slug rather than panicking)
- Failure visibility: `draw_status_bar` is a pure render function — no I/O; status bar degradation is `""` project name when `App.config` is `None`; `None` cycle_slug renders as blank (not "None")
- Redaction constraints: none — status bar displays only project name and milestone slug (no secrets)

## Integration Closure

- Upstream surfaces consumed: `App.config: Option<Config>` (added by S04 — `project_name` for status bar), `Screen::Settings` and `draw_settings` (added by S04 — needs `area: Rect` added to signature), `App` struct with all S01–S04 fields, `assay_core::milestone::cycle_status`
- New wiring introduced in this slice: global layout split in `draw()` threading `content_area` to all renderers; `draw_status_bar` + `draw_help_overlay` render functions; `?` key global handler; `Event::Resize` in `run()`; `cycle_slug` refresh in wizard and settings save paths
- What remains before the milestone is truly usable end-to-end: nothing — this slice is the milestone-completion slice; all S01–S04 work is now integrated; M006 done

## Tasks

- [x] **T01: Contract tests, App.show_help + App.cycle_slug fields, cycle_slug loading** `est:40m`
  - Why: Creates the test contract for all S05 behavior and adds the two App fields + loading logic needed by T02. Tests that require rendering changes (`?` key) compile but fail; tests that only require field presence or data loading pass immediately.
  - Files: `crates/assay-tui/tests/help_status.rs` (new), `crates/assay-tui/src/app.rs`
  - Do: (1) Create `tests/help_status.rs` with 6 tests using the same `key()` helper and `setup_project`-like fixture as `spec_browser.rs`. Tests: `show_help_starts_false` (assert `app.show_help == false` on fresh App); `question_mark_opens_help` (send `?` key, assert `show_help == true`); `question_mark_again_closes_help` (send `?` twice, assert `show_help == false`); `esc_closes_help_when_open` (set `show_help = true`, send `Esc`, assert `show_help == false` and not quit); `cycle_slug_none_for_draft_milestone` (setup project with `status = "draft"`, assert `app.cycle_slug == None`); `cycle_slug_some_for_in_progress_milestone` (setup project with `status = "in_progress"`, assert `app.cycle_slug == Some("alpha".to_string())`). (2) In `app.rs`, add `pub show_help: bool` (default `false`) and `pub cycle_slug: Option<String>` fields to the `App` struct. (3) Initialize both in `with_project_root`: `show_help: false`; `cycle_slug: assay_core::milestone::cycle_status(&assay_dir).ok().flatten().map(|cs| cs.milestone_slug)`. (4) Refresh `cycle_slug` in the wizard Submit success path (after `milestone_scan` reloads milestones): add `self.cycle_slug = assay_core::milestone::cycle_status(&assay_dir).ok().flatten().map(|cs| cs.milestone_slug);` immediately after `self.milestones = loaded;`. Import `assay_core::milestone::cycle_status` at the top of `app.rs`. (5) Confirm tests 1, 5, 6 pass; tests 2, 3, 4 compile but fail (no `?` handler yet).
  - Verify: `cargo test -p assay-tui --test help_status` compiles; tests 1, 5, 6 pass; tests 2, 3, 4 fail with assertion errors (not compile errors)
  - Done when: `tests/help_status.rs` compiles clean; 3 of 6 tests pass; `App` struct has `show_help` and `cycle_slug` fields; `cargo test -p assay-tui` (prior tests) still all pass

- [ ] **T02: Global layout split, status bar, help overlay, `?` key, resize fix, final just ready** `est:60m`
  - Why: Implements all remaining S05 deliverables. The global layout split is the structural prerequisite for status bar rendering; help overlay, `?` key, and resize are the behavior deliverables; `just ready` is the milestone gate.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/wizard.rs`, `crates/assay-tui/src/settings.rs` (S04), `crates/assay-tui/src/main.rs`
  - Do: (1) **Global layout split**: in `draw()`, replace per-screen `let area = frame.area();` with a top-level split: `let [content_area, status_area] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());`. Import `ratatui::layout::Rect`. Pass `content_area` to all per-screen render calls. (2) **Update `draw_*` signatures to accept `area: Rect`**: change `draw_no_project(frame)` → `draw_no_project(frame, area: Rect)`; `draw_load_error(frame, msg)` → `draw_load_error(frame, area: Rect, msg)`; `draw_dashboard(frame, milestones, list_state)` → `draw_dashboard(frame, area: Rect, milestones, list_state)`; `draw_milestone_detail(frame, milestone, list_state)` → `draw_milestone_detail(frame, area: Rect, milestone, list_state)`; `draw_chunk_detail(frame, chunk_slug, spec, note, run)` → `draw_chunk_detail(frame, area: Rect, chunk_slug, spec, note, run)`. In each function body, use the `area` parameter instead of `frame.area()`. In `wizard.rs`, update `draw_wizard(frame, state)` → `draw_wizard(frame, area: Rect, state)` and use `area` for the popup centering base rect. In S04's `settings.rs`, update `draw_settings` similarly. (3) **Status bar**: add `draw_status_bar(frame: &mut ratatui::Frame, area: Rect, project_name: &str, cycle_slug: Option<&str>)` free function in `app.rs`. Renders a single `Paragraph` with `Line::from(vec![Span::raw(project_name), Span::raw("  "), Span::raw(cycle_slug.unwrap_or("")).dim(), Span::raw("  ·  ? help · q quit").dim()])`. Call from `draw()` as `draw_status_bar(frame, status_area, project_name, self.cycle_slug.as_deref())` where `project_name = self.config.as_ref().map(|c| c.project_name.as_str()).unwrap_or("")`. (4) **Help overlay**: add `draw_help_overlay(frame: &mut ratatui::Frame, area: Rect)` free function in `app.rs`. Compute centered rect: `w = area.width.min(62); h = 22; x = area.x + (area.width.saturating_sub(w)) / 2; y = area.y + (area.height.saturating_sub(h)) / 2; let popup_area = Rect::new(x, y, w, h);`. Render `Clear` widget at `popup_area`. Render `Block::bordered().title(" Keybindings ")` at `popup_area`. Render `Table` with `Row`s for each keybinding: Global (`? help`, `q quit`), Dashboard (`↑↓ navigate`, `Enter open`, `n new`, `s settings`), Detail views (`Enter open chunk`, `Esc back`), Wizard (`Enter next`, `Backspace back/delete`, `Esc cancel`), Settings (`↑↓ select`, `w save`, `Esc cancel`). Add `use ratatui::widgets::Clear;`. Call `draw_help_overlay(frame, frame.area())` at the very end of `draw()` when `self.show_help` is true (after the screen match and status bar render). (5) **`?` key handler and help-overlay event guard**: at the top of `handle_event`, before the `match self.screen` block, add: `if self.show_help { if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) { self.show_help = false; } return false; }`. Then add: `if key.code == KeyCode::Char('?') && !matches!(self.screen, Screen::Wizard(_)) { self.show_help = true; return false; }`. (6) **cycle_slug refresh in Settings save path**: in the `Screen::Settings` arm of `handle_event`, in the `w` key success branch (where S04 transitions back to Dashboard), add: `if let Some(root) = &self.project_root { let assay_dir = root.join(".assay"); self.cycle_slug = assay_core::milestone::cycle_status(&assay_dir).ok().flatten().map(|cs| cs.milestone_slug); }`. (7) **Resize fix**: in `main.rs`, replace `if let Event::Key(key) = event::read()? && app.handle_event(key)` with a `match event::read()? { Event::Key(key) => { if app.handle_event(key) { break; } }, Event::Resize(..) => { terminal.clear()?; }, _ => {} }`. Add `use crossterm::event::Event;` import if not already present. (8) Run `just ready`; fix any fmt/clippy/deny issues. (9) Confirm all 6 `help_status` tests pass.
  - Verify: `cargo test -p assay-tui --test help_status` — all 6 pass; `cargo test -p assay-tui` — all prior tests pass; `just ready` — "All checks passed"
  - Done when: All 6 `help_status` tests pass; all prior assay-tui tests pass; `just ready` green; `draw_*` signatures all accept `area: Rect`; `Event::Resize` handled in `run()`

## Files Likely Touched

- `crates/assay-tui/tests/help_status.rs` — new: 6 contract tests for show_help and cycle_slug
- `crates/assay-tui/src/app.rs` — App fields (show_help, cycle_slug), cycle_status loading/refresh, global layout split, draw_* signature updates, draw_status_bar, draw_help_overlay, ? key handler, help-overlay event guard
- `crates/assay-tui/src/wizard.rs` — draw_wizard signature update (add area: Rect)
- `crates/assay-tui/src/settings.rs` — draw_settings signature update (add area: Rect) [added by S04]
- `crates/assay-tui/src/main.rs` — Event::Resize handling
