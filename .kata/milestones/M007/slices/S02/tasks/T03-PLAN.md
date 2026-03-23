---
estimated_steps: 7
estimated_files: 2
---

# T03: Extend Settings screen with model text-input fields and finalize

**Slice:** S02 — Provider Dispatch and Harness Wiring
**Milestone:** M007

## Description

Extend `Screen::Settings` with four new fields (`planning_model: String`,
`execution_model: String`, `review_model: String`, `model_focus: Option<usize>`),
update the `s` key handler to pre-populate buffers from config, add model-field rendering
to `draw_settings`, handle Tab/char/Backspace events for in-screen editing, update `w` save
to use in-screen model buffers, and verify the entire slice is clean with `just ready`.

Two new tests are written first (red), then the implementation makes them green.

## Steps

1. **Write failing model-field tests** in `crates/assay-tui/tests/settings.rs` (append to end):
   - `settings_model_fields_prepopulated_from_config`: (a) create a tmpdir project with a
     `config.toml` that includes `[provider]` section with `planning_model = "claude-3-haiku"`;
     (b) open `App::with_project_root(Some(root))`; (c) press `s` to open Settings;
     (d) destructure `Screen::Settings { planning_model, .. }` and assert it equals
     `"claude-3-haiku"`. This fails until the `Screen::Settings` variant is extended and
     the `s` key handler pre-populates the buffer.
   - `settings_w_save_includes_model_fields`: (a) create a tmpdir project with bare `config.toml`;
     (b) press `s`, then send char events `['t','e','s','t']` for the planning model (need to
     also send `Tab` first to focus the planning_model field), then press `w`; (c) reload config
     and assert `planning_model == Some("test")`. Tests the buffer-based save.
   - Confirm both tests fail with either a compile error or assertion panic before proceeding.

2. **Extend `Screen::Settings` variant** in `src/app.rs`:
   - Add four fields to the variant: `planning_model: String`, `execution_model: String`,
     `review_model: String`, `model_focus: Option<usize>`
   - Scan all `Screen::Settings` match arms in `app.rs` and add `..` to any pattern that only
     destructures `{ selected, error }` — the draw call and the Up/Down/Esc/w arms all need
     updating. Use `if let Screen::Settings { ref mut planning_model, .. }` pattern where needed.

3. **Update `s` key handler** to pre-populate model buffers:
   - Replace the existing `self.screen = Screen::Settings { selected, error: None }` with:
     ```rust
     let (pm, em, rm) = self.config.as_ref()
         .and_then(|c| c.provider.as_ref())
         .map(|p| (
             p.planning_model.clone().unwrap_or_default(),
             p.execution_model.clone().unwrap_or_default(),
             p.review_model.clone().unwrap_or_default(),
         ))
         .unwrap_or_default();
     self.screen = Screen::Settings {
         selected,
         error: None,
         planning_model: pm,
         execution_model: em,
         review_model: rm,
         model_focus: None,
     };
     ```

4. **Update `draw_settings` function** signature and body:
   - Add parameters for the new fields: `model_focus: Option<usize>`, `planning_model: &str`,
     `execution_model: &str`, `review_model: &str`
   - In the layout, split the existing `list_area` into a provider section (3 rows) and a model
     section (4 rows: one header + three fields); adjust `Constraint::Length` values
   - Render three model input rows with format `"  Planning model:  [<value>]"` where
     `[<value>]` shows the buffer contents; when that field is focused (`model_focus == Some(i)`),
     style the row cyan/bold; unfocused rows are dim
   - Update the call site in `draw()` to pass the new args (extract them from the `Screen::Settings` variant using `if let`)

5. **Update Settings event handler** for model focus and char input:
   - At the top of the `Screen::Settings` arm, before the `match key.code` block, check if
     `model_focus` is `Some`. When `model_focus.is_some()`:
     - `Tab` → cycle: `model_focus = Some((f + 1) % 3)`; when wrapping from 2 back to 0, cycle to `None` instead (returns to provider list focus)
     - `Esc` → `model_focus = None` (NOT leave Settings; just unfocus model section)
     - `Char(c)` → append `c` to the active model buffer: `if let Screen::Settings { ref mut planning_model, .. }` etc.
     - `Backspace` → pop last char from the active buffer
     - Other keys → no-op
     - Early return `false` after handling to skip the provider list navigation below
   - When `model_focus.is_none()`:
     - `Tab` → `model_focus = Some(0)` (enter model section)
     - All other existing keys (Up/Down/Esc/w/q) unchanged

6. **Update `w` save handler** to use in-screen model buffers instead of reading from `self.config`:
   - Replace the `planning_model: cfg.provider.as_ref().and_then(|p| p.planning_model.clone())` pattern with:
     ```rust
     planning_model: Some(planning_model_buf.clone()).filter(|s| !s.is_empty()),
     execution_model: Some(execution_model_buf.clone()).filter(|s| !s.is_empty()),
     review_model: Some(review_model_buf.clone()).filter(|s| !s.is_empty()),
     ```
     where `planning_model_buf` etc. are extracted from `Screen::Settings` before the existing
     `let kind = match selected {` line using the established borrow-safe pattern.

7. Run `cargo test -p assay-tui` — all tests pass (pre-existing 35 + 3 provider_dispatch + 2 new model tests). Run `just ready` — exits 0. Fix any `cargo fmt`, `cargo clippy`, or `cargo deny` issues before declaring done.

## Must-Haves

- [ ] `Screen::Settings` variant has all four new fields
- [ ] `s` key handler pre-populates model buffers from `app.config`; `model_focus` starts as `None`
- [ ] `draw_settings` renders three model input rows; focused field shown in cyan/bold
- [ ] `Tab` enters model section (first field); subsequent `Tab` cycles fields; `Tab` on last field returns to provider list (`model_focus = None`)
- [ ] `Char(c)` appends to active model buffer; `Backspace` pops last char
- [ ] `Esc` in model section clears focus (`model_focus = None`) without leaving Settings
- [ ] `w` save uses in-screen model buffers; empty buffer → `None` in `ProviderConfig`
- [ ] All pre-existing 35 settings tests pass (their `..` patterns absorb new fields)
- [ ] 2 new model-field tests pass
- [ ] `just ready` exits 0

## Verification

- `cargo test -p assay-tui --test settings` → all settings tests pass (including 2 new model tests)
- `cargo test -p assay-tui` → all tests pass (≥40 total)
- `just ready` → exit 0 (fmt + clippy + test + deny)

## Observability Impact

- Signals added/changed: Settings screen now shows model values from config in the UI; `w` save persists them; no new runtime observability signals (Settings is a configuration surface, not a stateful runtime)
- How a future agent inspects this: `assay_core::config::load(root)` reads the saved config; `config.provider.{planning,execution,review}_model` are the persisted values
- Failure state exposed: `w` save failure path (no project root or no config) already shows inline error in `Screen::Settings { error }` — unchanged; no new failure paths introduced

## Inputs

- `crates/assay-tui/tests/settings.rs` — existing 5 tests use `..` patterns; safe to append without breaking anything
- `origin/main:crates/assay-tui/src/app.rs` lines 270–280 (draw call), 432–446 (`s` key handler), 676–760 (Settings event handler) — exact code to update
- `S02-RESEARCH.md` — model focus navigation decisions, borrow-checker constraint pattern (D097/D098), cursor-based char input from `wizard.rs` reference, `w` save buffer pattern

## Expected Output

- `crates/assay-tui/src/app.rs` — `Screen::Settings` extended; all 6 code sites updated (variant definition, s-key, draw call, draw function, event handler, w-save)
- `crates/assay-tui/tests/settings.rs` — 2 new model-field tests appended
- `just ready` → exit 0 confirming the slice is clean and complete
