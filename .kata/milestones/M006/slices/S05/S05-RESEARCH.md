# S05: Help Overlay, Status Bar, and Integration Polish — Research

**Date:** 2026-03-21
**Confidence:** HIGH

## Summary

S05 is the integration-polish slice that completes M006. It has three concrete deliverables: (1) a `?`-triggered help overlay listing all keybindings, (2) a persistent bottom status bar showing project name, active milestone slug, and key hints, and (3) terminal-resize handling so the TUI doesn't go blank or corrupt after a `SIGWINCH`. A final `just ready` pass closes the milestone.

S04 is a hard prerequisite — it adds `App.config: Option<Config>` (which provides `project_name` for the status bar) and `Screen::Settings`. S05 must not be planned or started until S04's `with_project_root` loads config into `App`. The status bar's project-name segment should gracefully degrade to an empty string when `App.config` is `None` (no `.assay/config.toml` yet).

The codebase is healthy going into S05: `just ready` passes as of S03 (all checks green), the App/Screen architecture is clean (D089, D097–D100), and the 16 existing test points all pass.

## Recommendation

**Status bar via global layout split in `draw()`**: carve off the last 1-line row from `frame.area()` before passing the remaining area to each per-screen renderer. This is the least invasive approach — no existing render function needs to change its layout. The status bar renders project name + active milestone slug (loaded once at startup, refreshed after wizard submit) + dim key hints. Cache `cycle_status` result as `App.cycle_slug: Option<String>` — load it in `App::with_project_root` alongside `milestone_scan`, refresh it after wizard submit and after settings save.

**Help overlay as a centered popup**: reuse the `draw_wizard` popup geometry pattern (manual `Rect` centering + `Clear` + `Block` + `Table`). The overlay should be ~60×20 wide. Render it on top of whatever screen is active by calling the current-screen renderer first, then overlaying the help block. Mirror the wizard overlay pattern in D096 — render the base screen, then conditionally render the help block on top.

**Terminal resize**: the `main.rs` event loop currently only handles `Event::Key`; `Event::Resize` falls through silently. Ratatui auto-handles resize on the next `terminal.draw()` call, but only if the event loop **does not block** waiting for the next key. Fix: in the `run()` loop, match all `Event` variants; call `app.handle_event(key)` for `Event::Key`, and for `Event::Resize(_, _)` call `terminal.clear()` then continue (triggers immediate redraw). This eliminates any half-painted resize artifacts.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Clear area before popup | `ratatui::widgets::Clear` | Already used in `draw_wizard`; renders blank cells before the overlay block, preventing ghost text from the underlying screen showing through |
| Centered Rect for popup | Manual `Rect::new((area.width.saturating_sub(w))/2, ...)` | Established pattern from `draw_wizard` in `wizard.rs` — copy verbatim; avoids importing `Layout::flex` |
| Keybindings table | `ratatui::widgets::Table` + `Row` + `Cell` | Already imported and used in `draw_chunk_detail`; renders two-column key/action table with no extra deps |
| Global layout carve-out | `Layout::vertical([..., Constraint::Length(1)]).areas(area)` | Standard Ratatui pattern; `draw()` already does this per-screen; do it once at the top of `draw()` instead |
| Active milestone name | `assay_core::milestone::cycle::cycle_status(&assay_dir)` | Returns `Option<CycleStatus>` with `milestone_slug` + `milestone_name` — single call at startup |
| Config project name | `App.config.as_ref().map(|c| c.project_name.as_str())` | `App.config` added by S04; `Config.project_name: String` is the display field |

## Existing Code and Patterns

- `crates/assay-tui/src/app.rs` — `draw()` currently dispatches per-screen with no global chrome. Change: split `frame.area()` into `[content_area, status_area]` at the top of `draw()`, pass `content_area` to each per-screen renderer, render status bar in `status_area`.
- `crates/assay-tui/src/wizard.rs` — `draw_wizard` at line 303 renders full-screen (not a popup despite D096). S05's help overlay should follow the same full-screen layout but use a centred `Rect` + `Clear` pattern.
- `crates/assay-tui/src/main.rs` — `run()` at line 28: `if let Event::Key(key) = event::read()? && app.handle_event(key)`. Change to `match event::read()? { Event::Key(key) => if app.handle_event(key) { break }, Event::Resize(..) => terminal.clear()?, _ => {} }`.
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status(assay_dir)` returns `Result<Option<CycleStatus>>` synchronously (D007). Call once in `App::with_project_root`; store `Option<String>` slug in `App.cycle_slug`. Refresh in wizard submit path and settings save path.
- `crates/assay-tui/tests/spec_browser.rs` — `setup_project` helper pattern; follow for any new integration test in `tests/help_status.rs`.

## Constraints

- **D097/D098 borrow pattern is mandatory**: `draw_help_overlay(frame, area)` takes `Rect` explicitly (not `&mut App`); `draw_status_bar(frame, area, project_name: &str, cycle_slug: Option<&str>)` takes the two data slices as separate args.
- **S04 must be complete before S05 starts**: `App.config: Option<Config>` (for project_name) and `Screen::Settings` are added in S04. S05 reads `App.config` for status bar but does not add the field.
- **D001 zero-trait convention**: `draw_help_overlay` is a free function, not a `Widget` impl.
- **Status bar must not panic when `App.config` is `None`**: graceful degradation — render blank project name with key hints only.
- **Sync core (D007)**: `cycle_status` is sync; call it directly in `with_project_root`, not via spawn_blocking.
- **`just ready` must pass at S05 completion**: all four checks (fmt, lint, test, deny) green.

## Common Pitfalls

- **Forgetting to pass `content_area` (not `frame.area()`) to per-screen renderers after the global layout split.** If `draw_dashboard` is called with the full `frame.area()`, its `hint_area` will overlap the status bar. Fix: the global split in `draw()` produces `[content_area, status_area]`; pass `content_area` to all `draw_*` fns. All existing screen renderers accept an `area: Rect` argument (they call `frame.area()` internally today — change them to accept `area` as a parameter).

- **Help overlay rendered under the wizard popup.** If `draw()` renders help overlay before checking `Screen::Wizard`, the wizard covers the help. Fix: render help overlay last, after all screen content, so it sits on top of everything.

- **`cycle_status` called inside `terminal.draw()`.** All assay-core reads must happen in `handle_event()` or `App::new()`, not inside the draw callback (Pitfall 3 from M006 research). Store `cycle_slug: Option<String>` on `App`, refresh it at navigation transitions that change milestone state (after wizard submit, after settings save if milestone is advanced).

- **`Event::Resize` not matched in `run()`.** The current `if let Event::Key(key) = ...` pattern silently drops resize events. A resize followed immediately by `q` will appear as though `q` was lost (resize consumes the event read, next `event::read()` blocks). Fix with a proper `match`.

- **`draw_dashboard` and other renderers currently call `frame.area()` internally.** After the global layout split, they must accept an explicit `area: Rect` parameter. This is a 2-3 line change per renderer, but touching all 6 render functions at once is the reliable approach.

- **`?` key already claimed?** Verify: currently `?` has no handler in `handle_event`. It falls through to the no-op arm in Dashboard. Safe to add.

- **Status bar `cycle_slug` stale after wizard submit.** The wizard submit path in `handle_event` refreshes `self.milestones` via `milestone_scan`. It should also refresh `self.cycle_slug` via a second `cycle_status` call. Single-line addition.

## Open Risks

- **`draw_*` renderer signature change is mechanical but touches all 6 functions**: `draw_no_project`, `draw_load_error`, `draw_dashboard`, `draw_milestone_detail`, `draw_chunk_detail`, plus `draw_wizard` in `wizard.rs`. Each needs an `area: Rect` parameter. Low-risk but high line-count for a small change. Verify test suite stays green after refactor.

- **S04 timeline**: S05 cannot proceed until S04 is done. If S04 slips, S05's status bar will degrade to showing no project name (acceptable minimal version: status bar with empty project name slot, cycle slug if available, key hints). Plan for this by making `App.config` access always optional.

- **Help overlay size vs. terminal width**: the help table should be ~60 columns wide; terminals narrower than that will clip. Use `min(60, area.width.saturating_sub(4))` as the popup width. Not a correctness risk, just a visual degradation on very narrow terminals.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | (checked available_skills) | None found — no dedicated Ratatui skill in available_skills list |

## Sources

- `crates/assay-tui/src/app.rs` — current App struct, Screen enum, draw/handle_event architecture (HIGH confidence)
- `crates/assay-tui/src/main.rs` — current event loop, `Event::Key` only pattern (HIGH confidence)
- `crates/assay-tui/src/wizard.rs` — draw_wizard full-screen renderer, popup geometry approach (HIGH confidence)
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status()` API signature and return type (HIGH confidence)
- `~/.cargo/registry/src/.../ratatui-widgets-0.3.0/src/clear.rs` — `Clear` widget API (HIGH confidence)
- `~/.cargo/registry/src/.../crossterm-0.28.1/src/event.rs` line 562 — `Event::Resize(u16, u16)` variant (HIGH confidence)
- `.kata/milestones/M006/slices/S01/S01-SUMMARY.md` — App struct fields, Screen variants, borrow patterns (HIGH confidence)
- `.kata/milestones/M006/slices/S02/S02-SUMMARY.md` — wizard popup geometry, error-in-state pattern (HIGH confidence)
- `.kata/milestones/M006/slices/S03/S03-SUMMARY.md` — D097/D098/D099 patterns, detail_* fields (HIGH confidence)
- `.kata/milestones/M006/slices/S04/S04-PLAN.md` — App.config field, Screen::Settings coming in S04 (HIGH confidence)
