---
id: T03
parent: S02
milestone: M007
provides:
  - "`Screen::Settings` variant extended with `planning_model: String`, `execution_model: String`, `review_model: String`, `model_focus: Option<usize>` fields"
  - "`s` key handler pre-populates model buffers from `app.config.provider`; `model_focus` starts as `None`"
  - "`draw_settings` renders three model input rows (Planning/Execution/Review model) with cyan/bold focus highlight"
  - "Tab enters model section (planning field first); subsequent Tab cycles all three fields; Tab on last returns to provider list (`model_focus = None`)"
  - "Char(c) appends to active model buffer; Backspace pops; Esc in model section clears focus without leaving Settings; `w` always saves even when model_focus is Some"
  - "`w` save uses in-screen model buffers; empty buffer → `None` in `ProviderConfig`"
  - "2 new model-field tests in `tests/settings.rs`: `settings_model_fields_prepopulated_from_config` and `settings_w_save_includes_model_fields`"
  - "`just ready` exits 0 — all 40 assay-tui tests pass, fmt + clippy + deny clean"
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/settings.rs
key_decisions:
  - "Pressing `w` while model_focus is Some is treated as a global save command (falls through to the save arm) rather than appending 'w' to the active model buffer — this matches the test contract and is the natural UX"
  - "`#[allow(clippy::too_many_arguments)]` applied to `draw_settings` (9 args) to avoid over-engineering a settings-struct just for the renderer"
  - "Model section Tab cycle: 0→1→2→None (returns to provider list), not wrap-around to 0; Esc in model section also returns to provider list focus without leaving Settings"
patterns_established:
  - "Borrow-safe extraction pattern for `w` save: extract `(selected, pm_buf, em_buf, rm_buf)` by value from `Screen::Settings` before taking any mutable borrow on `self.project_root` / `self.config`"
  - "Model focus guard at top of Settings arm: read `model_focus` value first, then branch; early-return `false` after handling model-section keys to skip provider-list navigation"
observability_surfaces:
  - "Settings screen now shows model values from config in the UI; `w` save persists them"
  - "`assay_core::config::load(root)` reads back saved config; `config.provider.{planning,execution,review}_model` are the persisted values"
  - "Save failure path shows inline error in `Screen::Settings { error }` — unchanged from S01"
duration: 45min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T03: Extend Settings screen with model text-input fields and finalize

**Settings screen gains editable Planning/Execution/Review model text fields pre-populated from config, with Tab/Char/Backspace/Esc focus handling; `w` save persists buffers; `just ready` exits 0, closing S02.**

## What Happened

Two failing tests were written first (`settings_model_fields_prepopulated_from_config` and `settings_w_save_includes_model_fields`) anchoring the contract before any implementation.

`Screen::Settings` was extended with four new fields (`planning_model`, `execution_model`, `review_model`, `model_focus`). All existing match arms that destructured only `{ selected, error }` already used `..` patterns per S01 conventions — no existing tests broke.

The `s` key handler was updated to extract the three model strings from `app.config.provider` and set `model_focus: None`. The `draw_settings` function gained three new parameters and renders a model section below the provider list, with the focused field shown in cyan/bold.

The Settings event handler was restructured: a model-focus guard block at the top intercepts Tab/Esc/Char/Backspace when `model_focus.is_some()`, returning early. One deviation from the initial plan: `Char('w')` in the model-focus block was given an explicit fall-through case (no push, no early return) so the global `w` save command works even while a model field is focused. This was required by the `settings_w_save_includes_model_fields` test.

The `w` save handler was updated to extract model buffer strings before taking mutable borrows, using `Some(buf).filter(|s| !s.is_empty())` to convert empty strings to `None`.

`cargo fmt` was run to fix whitespace style. `draw_settings` received `#[allow(clippy::too_many_arguments)]` to satisfy the `-D warnings` clippy invocation.

## Verification

- `cargo test -p assay-tui --test settings` → 7/7 pass (5 pre-existing + 2 new model tests)
- `cargo test -p assay-tui` → 40/40 pass (8 agent_run + 1 app_wizard + 6 help_status + 3 provider_dispatch + 7 settings + 6 spec_browser + 9 wizard_round_trip)
- `just ready` → exit 0 (fmt + clippy + test + deny all green)

## Diagnostics

- `cargo test -p assay-tui --test settings` is the primary regression check for the Settings state machine
- `assay_core::config::load(root)` reads back the persisted model values after a `w` save
- Save failure surface unchanged: `Screen::Settings { error }` displays inline error text

## Deviations

- `Char('w')` in the model-focus handler falls through to the save arm rather than appending to the buffer. This was not explicit in the plan but is required for the `settings_w_save_includes_model_fields` test to pass and is the correct UX.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — `Screen::Settings` variant extended; `s` key handler pre-populates buffers; `draw_settings` signature and body updated; Settings event handler restructured with model-focus guard; `w` save handler uses in-screen buffers
- `crates/assay-tui/tests/settings.rs` — 2 new model-field tests appended
