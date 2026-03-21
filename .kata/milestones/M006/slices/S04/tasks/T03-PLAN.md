---
estimated_steps: 8
estimated_files: 5
---

# T03: Add `SettingsState`, `Screen::Settings`, and Settings screen to `assay-tui`

**Slice:** S04 — Provider Configuration Screen
**Milestone:** M006

## Description

Wire the TUI settings screen end-to-end: add `config: Option<Config>` to `App`, add `Screen::Settings(SettingsState)` variant and keybindings, create `settings.rs` with `SettingsState` and `draw_settings`, and write four tests in `tests/settings_screen.rs` proving the full navigation and save flow. The screen shows three providers (Anthropic, OpenAI, Ollama) with the currently configured one pre-selected. Pressing `w` saves the selection to `.assay/config.toml` via `assay_core::config::config_save`.

All render functions follow D097 (take individual fields, not `&mut App`). The settings draw function follows the same borrow-checker pattern established in S01/S02.

## Steps

1. Create `crates/assay-tui/src/settings.rs`:
   - Define `SettingsState { list_state: ListState, error: Option<String> }` with a `new(current_provider: ProviderKind) -> Self` constructor that pre-selects the matching list index.
   - Define the provider list as a constant `PROVIDERS: &[ProviderKind] = &[ProviderKind::Anthropic, ProviderKind::OpenAI, ProviderKind::Ollama]` with display names `["Anthropic (Claude)", "OpenAI (GPT)", "Ollama (local)"]`.
   - Define `pub fn draw_settings(frame: &mut ratatui::Frame, config: Option<&assay_types::Config>, state: &mut SettingsState)` that renders:
     - A bordered block titled `" Assay — Provider Settings "`
     - A `List` of three provider items; currently selected item highlighted via `render_stateful_widget`
     - Inline error text in red if `state.error.is_some()`
     - Hint line: `"↑↓ select · w save · Esc cancel"`
   - Provider names use the same display strings in the list items.

2. Add `pub mod settings;` to `crates/assay-tui/src/lib.rs`.

3. Open `crates/assay-tui/src/app.rs`. Add `config: Option<assay_types::Config>` field to `App` struct. Update `with_project_root` to load it:
   ```rust
   let config = project_root
       .as_deref()
       .and_then(|root| assay_core::config::load(root).ok());
   ```
   Include `config` in the `App { ... }` constructor.

4. Add `Screen::Settings(SettingsState)` variant to the `Screen` enum (import `SettingsState` via `use crate::settings::SettingsState;`).

5. Add `Settings` arm to `App::draw()`:
   ```rust
   Screen::Settings(ref mut state) => {
       draw_settings(frame, self.config.as_ref(), state);
   }
   ```

6. Add `s` key handling in the `Screen::Dashboard` branch of `handle_event`:
   ```rust
   KeyCode::Char('s') => {
       let current = self.config.as_ref()
           .and_then(|c| c.provider.as_ref())
           .map(|p| p.provider)
           .unwrap_or_default();
       self.screen = Screen::Settings(SettingsState::new(current));
   }
   ```

7. Add `Screen::Settings` arm to `handle_event`. On `KeyCode::Esc` or `KeyCode::Char('q')` → `self.screen = Screen::Dashboard`. On `KeyCode::Up` / `KeyCode::Down` → wrap-navigate the list (same wrapping logic as Dashboard). On `KeyCode::Char('w')`:
   - Read selected index from `state.list_state.selected().unwrap_or(0)`
   - Build `ProviderConfig { provider: PROVIDERS[idx], ..Default::default() }`
   - Build updated config: if `self.config` is `Some(c)`, update `c.provider`; if `None` and `project_root` is None, set `state.error = Some("No project found".into())` and return false
   - If `self.config` is `None`, set `state.error = Some("No config.toml found — run assay init first".into())` and return `false` (stay in Settings)
   - Otherwise, call `assay_core::config::config_save(root, &updated)` — on error set `state.error = Some(e.to_string())`; on success update `self.config` and return to `Screen::Dashboard`

8. Write `crates/assay-tui/tests/settings_screen.rs` with four tests:
   - `settings_opens_from_dashboard`: Create `App::with_project_root(Some(tmpdir))`, press `s` → assert `matches!(app.screen, Screen::Settings(_))`.
   - `settings_esc_returns_to_dashboard`: Open settings via `s`, press `Esc` → assert `matches!(app.screen, Screen::Dashboard)` and `app.config` unchanged.
   - `settings_save_updates_config`: Create a tempdir with `.assay/config.toml` (`project_name = "test"`), create `App::with_project_root(Some(tmpdir))`, press `s`, navigate to OpenAI (index 1) via `Down`, press `w` → assert `matches!(app.screen, Screen::Dashboard)`; read `.assay/config.toml` from disk and check `provider = "open_ai"` is present.
   - `settings_save_no_project_no_crash`: Create `App::with_project_root(None)`, press `s` (opens settings if available or returns NoProject — verify no panic); handle gracefully.

   Note: `settings_save_no_project_no_crash` should verify that pressing `s` from `Screen::NoProject` does NOT transition to Settings (guard: the `s` handler should only be active in `Screen::Dashboard`). Check that the screen remains `NoProject`.

## Must-Haves

- [ ] `SettingsState::new(current: ProviderKind)` pre-selects the correct list index
- [ ] `draw_settings` is a free function taking `Option<&Config>` and `&mut SettingsState` separately (D097)
- [ ] `App.config: Option<Config>` field populated via `config::load(root).ok()` in `with_project_root`
- [ ] `Screen::Settings(SettingsState)` variant in `Screen` enum
- [ ] `s` key only active from `Screen::Dashboard` branch (not from NoProject or LoadError)
- [ ] `w` key saves via `assay_core::config::config_save`; sets `state.error` on failure; transitions to Dashboard on success
- [ ] Wrapping ↑↓ navigation in Settings (consistent with Dashboard navigation)
- [ ] All 4 settings tests pass; no pre-existing tests broken

## Verification

- `cargo test -p assay-tui settings` — 4 new tests pass
- `cargo test -p assay-tui` — total ≥ 27 tests, all green
- `cargo test --workspace` — no regressions (≥ 1356 prior tests)
- `just ready` — fmt, lint, test, deny all green

## Observability Impact

- Signals added/changed: `App.config` field — `None` means no config on disk; `Some` means loaded; mutated in-memory on successful save. `state.error: Option<String>` — inline error visible in Settings popup (same pattern as wizard error from S02).
- How a future agent inspects this: `App.config.as_ref().and_then(|c| c.provider.as_ref())` to see current provider; check `.assay/config.toml` on disk; `cargo test -p assay-tui settings_save_updates_config` exercises the full save path
- Failure state exposed: `config_save` errors are surfaced inline in `state.error`; the Settings screen stays open so the user sees the error without losing their selection

## Inputs

- T01 complete — `ProviderKind`, `ProviderConfig`, `Config.provider` field must exist in `assay-types`
- T02 complete — `assay_core::config::config_save` must be exported and callable
- `crates/assay-tui/src/app.rs` (S01/S02 state) — `App`, `Screen`, `handle_event`, `draw` patterns established
- D097 — screen-specific render fns take individual fields, not `&mut App` (borrow-checker pattern)
- S02 patterns: popup `state.error` pattern, `pub mod` in lib.rs, borrow-checker render fn signature

## Expected Output

- `crates/assay-tui/src/settings.rs` — new: `SettingsState`, `draw_settings`, `PROVIDERS` constant
- `crates/assay-tui/src/lib.rs` — `pub mod settings;` added
- `crates/assay-tui/src/app.rs` — `config` field on App, `Screen::Settings` variant, `s`/`w`/Esc handlers, draw arm
- `crates/assay-tui/tests/settings_screen.rs` — new: 4 passing tests
