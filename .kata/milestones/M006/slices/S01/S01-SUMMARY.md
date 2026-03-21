---
id: S01
parent: M006
milestone: M006
provides:
  - "[[bin]] name = 'assay-tui' in Cargo.toml — explicit binary declaration; assay-tui and assay binaries coexist without collision"
  - "lib.rs with App, Screen, WizardState, draw, handle_event, run as public API — testable via tests/"
  - "Screen enum with 6 variants: Dashboard, MilestoneDetail, ChunkDetail, Wizard(WizardState), Settings, NoProject"
  - "App struct: screen, milestones, list_state, project_root, config, show_help"
  - "draw_dashboard renders bordered List with name/status badge/chunk progress from milestone_scan"
  - "draw_no_project renders centered message when .assay/ absent — no panic, clean exit on q"
  - "Wrapping ↑↓ navigation, Enter→MilestoneDetail, Esc→Dashboard, q→quit (bool-return protocol)"
  - "Empty-list guard in draw_dashboard prevents ListState panic on projects with zero milestones"
  - "7 unit tests in tests/app_state.rs — all state transitions covered without terminal"
  - "aws-lc-rs 1.16.2 + rustls-webpki 0.103.10 — RUSTSEC advisories cleared; just ready green"
requires: []
affects:
  - S02
  - S03
  - S04
  - S05
key_files:
  - crates/assay-tui/Cargo.toml
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/tests/app_state.rs
  - Cargo.lock
key_decisions:
  - "D088 — assay-tui binary is named assay-tui (not assay); [[bin]] explicit declaration required"
  - "D089 — App struct + Screen enum architecture; free draw/handle_event functions (D001 compliance)"
  - "D094 — lib.rs + thin main.rs split so tests/ can import from assay_tui::"
  - "D095 — Screen-specific render fns take separate fields (not &mut App) to satisfy borrow checker while using stateful widgets"
  - "Config imported from assay_types::Config (assay_core::config has no Config struct — it uses assay_types::Config internally)"
patterns_established:
  - "discriminant-first borrow pattern: matches!(app.screen, Screen::X) early-return guard + pass individual fields to render fn"
  - "thin main.rs entry point pattern: color_eyre::install() → ratatui::init() → App construction → assay_tui::run() → ratatui::restore()"
  - "handle_event returns bool (false = quit) as single control-flow signal; no exceptions"
  - "milestone_scan errors degrade gracefully to vec![] via unwrap_or_default(); no panic path for corrupt milestone files"
observability_surfaces:
  - "App.screen variant — inspect to know current view; readable in tests and debugger"
  - "App.milestones.len() — 0 if no .assay/ or no milestone TOML files"
  - "App.config.is_some() — true if config::load succeeded"
  - "tests/app_state.rs — 7 tests serve as executable spec for all key→state transitions; cargo test -p assay-tui"
  - "handle_event bool return — false means quit was requested; only q triggers this"
drill_down_paths:
  - .kata/milestones/M006/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M006/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M006/slices/S01/tasks/T03-SUMMARY.md
duration: 55min
verification_result: passed
completed_at: 2026-03-20T22:00:00Z
---

# S01: App Scaffold, Dashboard, and Binary Fix

**Replaced the 42-line assay-tui stub with a full Ratatui application scaffold: explicit [[bin]] declaration, App/Screen type hierarchy, live dashboard loading from milestone_scan, wrapping keyboard navigation, no-project guard, empty-list guard, and 7 passing unit tests — just ready green.**

## What Happened

**T01** addressed the structural foundation. The original `assay-tui` crate was a 42-line main.rs stub with no `[[bin]]` section — it produced no named binary. T01 added `[[bin]] name = "assay-tui"` to Cargo.toml (D088), split the crate into `lib.rs` + thin `main.rs` (D094), and defined the full `App`/`Screen`/`WizardState` type hierarchy with placeholder render fns. One early discovery: `Config` lives in `assay_types::Config`, not `assay_core::config::Config` — the core config module has no `Config` struct of its own; it uses the types crate internally. `assay-types` was added as a workspace dependency (it was missing from assay-tui's Cargo.toml).

**T02** wired in live data. `main()` now detects `.assay/` presence, calls `milestone_scan(&assay_dir)` and `config::load(&project_root)` with correct path contracts (scan gets the `.assay/` subdir; load gets the project root), and stores results in `App`. `draw_dashboard` renders a bordered `List` with each milestone formatted as `"{name}  [{badge}]  {done}/{total}"` via `render_stateful_widget`. `draw_no_project` renders a bold-red centered paragraph with a quit hint. The borrow-checker pattern emerged here: passing `&[Milestone]` and `&mut ListState` separately (D095) rather than `&mut App` is the correct Ratatui pattern for stateful widgets.

**T03** completed the interactive behavior and test coverage. Navigation was changed from clamping (T02's `.min`/`saturating_sub`) to wrapping — both Down and Up wrap at list boundaries. Enter transitions to `Screen::MilestoneDetail` (stub). Esc returns to Dashboard from any non-Dashboard screen. The empty-list guard in `draw_dashboard` (`if milestones.is_empty() { render placeholder; return }`) prevents the `ListState` panic for projects with no milestone files. Seven unit tests in `tests/app_state.rs` cover every navigation transition without requiring a terminal. `cargo update` was run to bump aws-lc-rs and rustls-webpki past pre-existing RUSTSEC advisories before `just ready` could pass.

## Verification

- `cargo build -p assay-tui` → Finished; `target/debug/assay-tui` exists (11 MB)
- `cargo build -p assay-cli` → Finished; `target/debug/assay` exists (41 MB); no binary collision
- `cargo test -p assay-tui` → `test result: ok. 7 passed; 0 failed`
- `just ready` → fmt ✓, lint ✓, test ✓ (1331+ workspace tests), deny ✓ — "All checks passed"
- `[[bin]] name = "assay-tui"` confirmed present in Cargo.toml before `[dependencies]`
- No `set_hook`/`take_hook` in main.rs (ratatui::init() owns the panic hook)

## Requirements Advanced

- R049 (TUI project dashboard) — S01 establishes the runtime foundation: real binary, live milestone data from milestone_scan, keyboard navigation. R049 is now *partially validated* — the dashboard renders real data; spec browser detail (S03) and agent spawning (M007) remain.

## Requirements Validated

- None fully validated by this slice alone — R049 requires S03 (chunk detail) and S05 (help/status bar) to be fully proved.

## New Requirements Surfaced

- None. The scope matched planning.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

- `Config` imported from `assay_types::Config` instead of the plan's `assay_core::config::Config` — the core config module has no `Config` struct; it internally uses `assay_types::Config`. Corrected in T01.
- `assay-types` added to `assay-tui/Cargo.toml` — missing from the original file; required for `Milestone` and `Config`. Discovered in T01.
- ↑↓ navigation implemented in T02 (plan put it in T03) — needed for the list to be functionally usable. No impact on slice scope.
- Navigation changed from clamping to wrapping in T03 — T02 implemented clamping; T03 corrected to the specified wrapping semantics.
- `cargo update -p aws-lc-rs` and `cargo update -p rustls-webpki` run in T03 to clear pre-existing RUSTSEC advisories that blocked `just ready`. Not in the task plan; required for the slice-level verification to pass.

## Known Limitations

- `Screen::MilestoneDetail`, `Screen::ChunkDetail`, `Screen::Wizard`, `Screen::Settings` all render placeholder text — real content deferred to S02–S04.
- No `show_help` / help overlay rendering yet — `App.show_help` field exists but is unused until S05.
- No status bar — deferred to S05.
- `milestone_scan` errors are silently swallowed via `unwrap_or_default()` — a corrupt milestone TOML shows as an absent milestone with no diagnostic message in the TUI (a known graceful-degradation tradeoff).

## Follow-ups

- S02: implement `WizardState` fields and `draw_wizard`/`handle_wizard_event` — `Screen::Wizard(WizardState)` variant slot already exists; `App.project_root` available for `create_from_inputs`.
- S03: implement `Screen::MilestoneDetail` + `Screen::ChunkDetail` with real data loading on navigation — Enter dispatch and `App.project_root` already wired.
- S04: implement `Screen::Settings` — `App.config` already loaded; `config_save` (D093) will be added to `assay-core::config`.
- S05: implement help overlay using `App.show_help` field + status bar.

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added `[[bin]]` section, `assay-types.workspace = true`, `[dev-dependencies]` with `chrono`
- `crates/assay-tui/src/lib.rs` — new; all pub types (App, Screen, WizardState) and functions (draw, handle_event, run, draw_dashboard, draw_no_project)
- `crates/assay-tui/src/main.rs` — rewritten as 12-line thin entry point with real data loading
- `crates/assay-tui/tests/app_state.rs` — new; 7 unit tests for all state transitions
- `Cargo.lock` — aws-lc-rs 1.16.0→1.16.2, aws-lc-sys 0.37.1→0.39.0, rustls-webpki 0.103.9→0.103.10

## Forward Intelligence

### What the next slice should know
- `App.project_root` is `Option<PathBuf>` pointing at the project root (parent of `.assay/`). Pass `app.project_root.as_deref().unwrap()` to `create_from_inputs` in S02 and `milestone_load` / `history::load` in S03.
- `Screen::Wizard(WizardState)` variant already exists but `WizardState` is a stub (`Default`-derived empty struct). S02 must replace `WizardState`'s fields in place — do not add a new type.
- The borrow-checker pattern (D095) is established: all screen-specific render fns take individual fields (`&[Milestone]`, `&mut ListState`), not `&mut App`. Follow this for `draw_wizard`, `draw_milestone_detail`, `draw_chunk_detail`, `draw_settings`.
- `config::load` is already called in `main()` and stored in `App.config`. S04 only needs to add `config_save` to `assay-core::config` and wire a Settings screen mutating `App.config`.

### What's fragile
- `draw_dashboard` empty-list guard: the guard (`if milestones.is_empty()`) must precede the `ListItem` construction loop. If S03 or S05 refactors `draw_dashboard`, preserve this guard — removing it causes a `ListState` panic when milestones are empty.
- `handle_event` Screen dispatch: the `matches!` early-return pattern in `draw()` is fragile if new screens are added without also adding a corresponding arm. S02–S05 must add both a `draw_*` fn AND a `matches!` arm in `draw()`.

### Authoritative diagnostics
- `cargo test -p assay-tui` — fastest way to confirm all navigation invariants still hold after edits; 7 tests cover every key→state transition
- `App.screen` field — inspect in a debugger or test assertion to determine current view
- `just ready` — the final gate before marking a slice complete; all four checks (fmt, lint, test, deny) must pass

### What assumptions changed
- Original assumption: `assay_core::config` exports a `Config` struct. Actual: it uses `assay_types::Config`. Any code in S04 that needs `Config` must import from `assay_types`, not `assay_core::config`.
- Original assumption: `assay-types` was already a dependency of `assay-tui`. Actual: it was missing and had to be added.
