---
id: M006
provides:
  - "assay-tui full Ratatui application (replaces 42-line stub): live dashboard, in-TUI wizard, spec browser, provider config screen, help overlay, status bar"
  - "[[bin]] name = 'assay-tui' — explicit binary declaration; assay-tui and assay binaries coexist without collision"
  - "App struct + Screen enum (Dashboard/NoProject/Wizard/MilestoneDetail/ChunkDetail/Settings/LoadError) as canonical TUI state machine"
  - "draw_dashboard: bordered list with milestone name, status badge, chunk progress fraction, live from milestone_scan"
  - "Screen::NoProject guard: clean message on missing .assay/, no panic"
  - "WizardState pure state machine + draw_wizard popup: multi-step form (name→chunk count→chunk names→criteria) → create_from_inputs → milestone appears in dashboard"
  - "wizard_round_trip integration test: synthetic KeyEvents → create_from_inputs → tempdir file assertions"
  - "Screen::MilestoneDetail + Screen::ChunkDetail: Enter→milestone→chunk→criteria table; Esc chains; wrapping navigation"
  - "join_results: GatesSpec.criteria joined against GateRunRecord.summary.results by criterion name → Vec<(&Criterion, Option<bool>)>"
  - "ProviderKind enum (Anthropic|OpenAi|Ollama) and ProviderConfig struct in assay-types with serde(default, skip_serializing_if) — D092 backward-compat"
  - "Config.provider: Option<ProviderConfig> — existing config.toml without [provider] loads without error"
  - "config_save (save fn) in assay-core::config using NamedTempFile+sync_all+persist — D093 atomic write"
  - "Screen::Settings: full-screen provider list, ↑↓ selection, w saves to .assay/config.toml, Esc cancels"
  - "Provider config round-trips through .assay/config.toml; survives TUI restart"
  - "App.show_help: bool toggle; ? key from any non-wizard screen; draw_help_overlay centered popup"
  - "draw_status_bar: project name · cycle slug · key hints — persistent bottom line"
  - "Global layout split: App::draw splits frame.area() into [content_area, status_area]; all draw_* accept area: Rect"
  - "Event::Resize handler in run() calling terminal.clear()"
  - "just ready passing (fmt, lint, test, deny); 1367+ workspace tests"
key_decisions:
  - "D088 — [[bin]] name = 'assay-tui'; assay-cli keeps name = 'assay'"
  - "D089 — App struct + Screen enum architecture; free draw/handle_event functions (D001)"
  - "D090 — WizardState nested in Screen::Wizard variant; wizard is TUI concern only"
  - "D091 — synchronous data loading on navigation transitions; no background thread for S01–S04"
  - "D092 — ProviderConfig follows D056 pattern exactly: serde(default, skip_serializing_if) on all fields"
  - "D093 — config_save uses NamedTempFile atomic write pattern consistent with milestone_save"
  - "D094 — lib.rs + thin main.rs split for testability; combined bin+lib crate pattern"
  - "D095 — Screen-specific render fns take individual fields, not &mut App (borrow checker + stateful widgets)"
  - "D096 — draw() overlays wizard popup on top of Dashboard for borrow-split avoidance"
  - "D097 — discriminant-first borrow pattern: early-return guards + pass individual fields to render fn"
  - "D098 — .. pattern in draw() match arms; clone-then-mutate in handle_event() for slug reads"
  - "D099 — App-level detail_* fields for loaded data; preserves detail_list_state across Esc transitions"
  - "D100 — Criterion/gate-run join by exact name match; unmatched → None (Pending)"
  - "D101 — Settings screen 'w' key saves; Esc/q cancel"
  - "D102 — Settings screen is full-screen (not popup)"
  - "D103 — save with no loaded config shows inline error; no panic"
  - "D104 — help overlay event guard: only ? or Esc dismiss while show_help=true"
  - "D105 — all draw_* accept explicit area: Rect; App::draw splits once at top"
  - "D106 — App.cycle_slug cached; refreshed only at lifecycle transitions"
patterns_established:
  - "discriminant-first borrow pattern: matches!(app.screen, Screen::X) early-return guard + pass individual fields to render fn"
  - "thin main.rs entry point: color_eyre::install() → ratatui::init() → App construction → assay_tui::run() → ratatui::restore()"
  - "handle_event returns bool (false = quit) as single control-flow signal"
  - "milestone_scan errors degrade gracefully to vec![] via unwrap_or_default()"
  - "pure event function pattern: handle_*_event takes &mut State + KeyEvent, returns owned Action enum"
  - "popup overlay pattern: render Clear first, then Block, then content; set_cursor_position at end"
  - "error-in-state pattern: state.error: Option<String> set on failure, cleared on next keypress"
  - "draw() always renders Dashboard first, then conditionally overlays wizard popup (D096)"
  - "global layout split pattern: App::draw splits frame.area() into [content_area, status_area] first"
  - "overlay-last rendering: draw_help_overlay called after all screen renderers"
  - "sorted-chunk index derivation: clone chunks, sort_by_key(order), index — consistent across draw and handle_event"
observability_surfaces:
  - "App.screen variant — inspect to know current view; readable in tests and debugger"
  - "App.milestones.len() — 0 if no .assay/ or no milestone TOML files"
  - "App.config.is_some() — true if config::load succeeded on startup"
  - "App.show_help — true = overlay visible; false = normal navigation"
  - "App.cycle_slug — Some(slug) when InProgress milestone exists; None otherwise"
  - "App.detail_spec_note.as_deref() — canonical reason string when detail_spec is None"
  - "Screen::LoadError(msg) — surfaces milestone_load failures inline"
  - "Screen::Settings { selected, error } — selected=0/1/2 for Anthropic/OpenAI/Ollama; error=None on clean state"
  - "cargo test -p assay-tui — 27 tests serve as executable spec for all key→state transitions"
requirement_outcomes:
  - id: R049
    from_status: active
    to_status: validated
    proof: "assay-tui binary produced (target/debug/assay-tui, 13MB); App::new() with Screen::NoProject guard (unit test); draw_dashboard renders milestone name + status badge + chunk fraction from milestone_scan; handle_event Up/Down/q/Enter tested; 6 spec_browser integration tests prove navigation; just ready green"
  - id: R050
    from_status: active
    to_status: validated
    proof: "WizardState pure state machine (handle_wizard_event) + draw_wizard popup (centered 64×14, hardware cursor); App wiring (n→wizard→Dashboard); wizard_round_trip integration test drives synthetic KeyEvents through N=2 chunk flow → create_from_inputs → asserts milestone TOML + two gates.toml files written to tempdir; 23 assay-tui tests pass"
  - id: R051
    from_status: active
    to_status: validated
    proof: "Dashboard→MilestoneDetail→ChunkDetail navigation with Esc chains; criteria Table with ✓/✗/? icons from join_results; empty history renders as all Pending; Legacy spec shows fallback message; 6 spec_browser integration tests all pass (enter_on_dashboard_navigates_to_milestone_detail, up_down_in_milestone_detail, esc_from_milestone_detail, enter_on_chunk_navigates_to_chunk_detail, esc_from_chunk_detail, chunk_detail_no_history_all_pending); just ready green"
  - id: R052
    from_status: active
    to_status: validated
    proof: "ProviderKind+ProviderConfig in assay-types with serde(default, skip_serializing_if) per D092; config_save (save fn) in assay-core::config with NamedTempFile+sync_all+persist; config_toml_roundtrip_without_provider test proves backward compat; Screen::Settings full-screen view with ↑↓ provider selection; w key saves to .assay/config.toml; 5 settings integration tests pass (s_key_opens_settings, esc_returns_to_dashboard, arrow_keys_cycle, w_saves_provider, saved_provider_survives_restart); provider schema snapshots locked; just ready green"
duration: ~9h total (S01: 55min, S02: ~3.5h, S03: 75min, S04: ~1.5h, S05: ~70min, milestone close: ~30min)
verification_result: passed
completed_at: 2026-03-21
---

# M006: TUI as Primary Surface

**Replaced the 42-line assay-tui stub with a full 5-screen Ratatui application: live dashboard, in-TUI authoring wizard, spec browser with criterion/gate-result join, provider configuration screen, and integration polish (help overlay, status bar, resize handling) — all 4 requirements validated, 1367 tests passing, just ready green.**

## What Happened

Five slices delivered the TUI in dependency order. The milestone was complicated by working-directory changes (S04's backend types) that accumulated without being committed; the milestone close step committed those changes alongside the completed Settings screen UI.

**S01** laid the foundation that all other slices consumed. It added the explicit `[[bin]] name = "assay-tui"` declaration to resolve the binary name conflict with `assay-cli` (D088), split the crate into `lib.rs` + thin `main.rs` (D094), and defined the full `App`/`Screen` type hierarchy. The `draw_dashboard` function rendered live data from `milestone_scan` into a bordered `List`. The borrow-checker pattern of passing individual fields to render functions (not `&mut App`) emerged here as a foundational constraint for Ratatui stateful widgets (D097). Seven unit tests in `tests/app_state.rs` proved all navigation transitions.

**S02** tackled the highest-risk slice: the multi-step form state machine. `WizardState` tracks `step`, `fields: Vec<Vec<String>>`, `cursor`, and `chunk_count`. The pure `handle_wizard_event` function dispatches all input without side effects — Char/Backspace/Enter per step type, ChunkCount replace-semantics for single-digit input, dynamic field allocation when chunk count is confirmed, and `assemble_submit()` building `WizardInputs` with `slugify()`. The `wizard_round_trip` integration test drives synthetic `KeyEvent`s through a complete 2-chunk flow, calls `create_from_inputs`, and asserts the milestone TOML and two `gates.toml` files exist in a tempdir.

**S03** extended navigation into two new full-screen views. Dashboard `Enter` calls `milestone_load`, populates five `App.detail_*` fields (following D099 — app-level storage, not Screen-variant embedding), and transitions to `Screen::MilestoneDetail`. MilestoneDetail `Enter` calls `load_spec_entry_with_diagnostics` + `history::list`+`load`, then transitions to `Screen::ChunkDetail`. The `join_results` function joins `GatesSpec.criteria` against `GateRunRecord.summary.results` by criterion name (D100), producing `Option<bool>` per criterion (None = Pending, Some(true) = pass, Some(false) = fail). Six `spec_browser` integration tests proved the full navigation graph including edge cases (no history, Legacy spec).

**S04** delivered the backend types and the Settings screen UI. The backend work (`ProviderKind`/`ProviderConfig` in `assay-types`, `config_save`/`save` in `assay-core::config`, schema snapshots, and round-trip tests in `config_provider.rs`) had accumulated in the working directory but was uncommitted at milestone-close time. The Settings screen — `Screen::Settings { selected, error }`, `draw_settings` full-screen renderer, `s` key from Dashboard, `↑↓` provider selection, `w` atomic save, `Esc` cancel — was implemented during milestone close alongside committing all backend work. Five settings integration tests proved open/close/navigate/save/restart-persistence.

**S05** delivered integration polish. It refactored all `draw_*` signatures to accept `area: Rect` (D105), adding a single global layout split in `App::draw` that produces `[content_area, status_area]`. `draw_status_bar` renders project name · cycle slug · key hints as a dim single-line `Paragraph`. `draw_help_overlay` renders a centered 62×22 popup with a two-column keybinding `Table`. The `?` key handler guard (D104) ensures only `?`/`Esc` dismiss the overlay while it's open. `Event::Resize` triggers `terminal.clear()` to prevent artifacts. `App.cycle_slug` is loaded on startup and refreshed only at lifecycle transitions (wizard submit, settings save) per D106.

The milestone close step also accepted the outstanding schema snapshots (`config-schema`, `provider-kind-schema`, `provider-config-schema`, `run-manifest-schema`) that had accumulated as `.snap.new` files, making `just ready` pass cleanly.

## Cross-Slice Verification

**Success criterion: dashboard renders live data from assay-core**
- `cargo test -p assay-tui --test spec_browser` — `enter_on_dashboard_navigates_to_milestone_detail` drives `handle_event(Enter)` on a real `.assay/` fixture and confirms the screen transitions to `Screen::MilestoneDetail`
- `draw_dashboard` calls `milestone_scan` (not hardcoded data); verified in `App::with_project_root` implementation

**Success criterion: navigating into milestone → chunks → criteria + gate results**
- 6 `spec_browser` tests prove the full graph: enter dashboard → milestone detail → chunk detail → esc back → esc back
- `chunk_detail_no_history_all_pending` confirms `join_results` returns `None` (Pending) for all criteria when no gate run history exists

**Success criterion: pressing `n` launches wizard; completing it creates real files**
- `wizard_round_trip` integration test: synthetic `KeyEvent`s drive through all steps → `create_from_inputs` → asserts `alpha.toml` and `specs/chunk-one/gates.toml` + `specs/chunk-two/gates.toml` written to tempdir
- App tests: `n` key opens `Screen::Wizard`; `Esc` returns to Dashboard without writing; slug collision sets `state.error` and stays in wizard

**Success criterion: pressing `s` opens settings; choice persists to config.toml**
- 5 `settings` tests prove: s-key opens Settings; Esc returns without saving; ↑↓ cycles selection; `w` saves to disk; provider survives a simulated TUI restart (App reconstructed from same tempdir)
- `w_saves_provider_and_returns_to_dashboard`: calls `assay_core::config::load` directly after save to confirm the TOML file contains the expected provider

**Success criterion: no panic on missing .assay/**
- `App::with_project_root(None)` → `Screen::NoProject`; unit test in `app_wizard.rs` confirms no panic
- `draw_no_project` renders bold-red message with quit hint; `q` exits cleanly

**Success criterion: `just ready` passes; binary names correct**
- `cargo build -p assay-tui` → `target/debug/assay-tui` (13MB)
- `cargo build -p assay-cli` → `target/debug/assay` (41MB)
- `just ready` → fmt ✓, lint ✓, test ✓ (1367 tests, 0 failures), deny ✓ — "All checks passed"

## Requirement Changes

- R049: active → validated — `assay-tui` binary produced; live dashboard from `milestone_scan`; keyboard navigation unit tests; NoProject guard; `just ready` green
- R050: active → validated — wizard_round_trip integration test proves end-to-end file creation; App key-wiring tests; 23 assay-tui tests pass
- R051: active → validated — 6 spec_browser integration tests prove full navigation graph; join_results criterion/gate join; Pending fallback for no history
- R052: active → validated — ProviderConfig/ProviderKind types + config_save backend; Screen::Settings full-screen UI; 5 settings integration tests including restart-persistence; schema snapshots locked; backward-compat round-trip test

## Forward Intelligence

### What the next milestone should know
- `App.config` is now `Option<assay_types::Config>` (the real type, not the placeholder `AppConfig` from S05's first pass). Any M007 code that reads `App.config` gets the full `Config` including `provider`.
- `Screen::Settings { selected, error }` uses `selected: usize` (0=Anthropic, 1=OpenAI, 2=Ollama). If M007 adds provider-specific model input fields, extend `SettingsState` as a struct moved into the variant (same pattern as `Screen::Wizard(WizardState)`).
- `draw_settings` currently shows provider selection only (no model input fields). The S04 plan's model-per-phase fields (`planning_model`, `execution_model`, `review_model`) are in `ProviderConfig` but the Settings UI only toggles `provider`. Editing models requires manual `config.toml` editing or a Settings UI extension in M007.
- `config_save` is `assay_core::config::save(root, &cfg)` — not `config_save`. The rename from the D093 plan was intentional (matches the `load` companion function naming).
- All `draw_*` functions accept `area: Rect` as second parameter. Any new screen renderer added in M007 must follow this pattern.
- `App.cycle_slug` is refreshed after wizard Submit and settings `w` save. If M007 adds live gate execution that changes milestone state, add a refresh there too.

### What's fragile
- `draw_dashboard` empty-list guard: `if milestones.is_empty()` must precede `ListItem` construction. Removing it causes a `ListState` panic on zero milestones.
- `handle_event` Screen dispatch: the `matches!` early-return pattern in `draw()` is load-bearing. New screen variants require both a `draw_*` fn AND a corresponding arm in `draw()`'s `match &self.screen`.
- `join_results` uses linear scan (O(n²) in criteria count). Fine for ≤15 criteria; needs attention if spec sizes grow.
- Schema snapshots locked: `config-schema.snap`, `provider-kind-schema.snap`, `provider-config-schema.snap`, `run-manifest-schema.snap`. Any change to `Config`, `ProviderKind`, `ProviderConfig`, or `RunManifest` requires `cargo insta accept` before `just ready` can pass.
- `setup_project` fixture in `spec_browser.rs` requires at least one criterion with `cmd = "true"`. Test changes that remove the cmd field will cause `LoadError` during navigation.

### Authoritative diagnostics
- `cargo test -p assay-tui` — 27 tests covering all navigation invariants; fastest diagnostic after any TUI code change
- `App.screen` field — inspect in tests or debugger to determine current view
- `app.detail_spec_note.as_deref()` — reason string when `detail_spec` is `None`; check first when ChunkDetail shows "No spec data"
- `grep "frame.area()" crates/assay-tui/src/app.rs` — should return exactly 2 lines (layout split + overlay call); extra matches indicate a `draw_*` helper broke the area contract
- `just ready` — four-check gate before marking any slice complete

### What assumptions changed
- S04's Screen::Settings was planned to be implemented in its own slice PR but the backend types (ProviderConfig, ProviderKind, config_save, schema tests) accumulated in the working directory without being committed. The milestone-close step caught this gap and implemented the Settings screen UI alongside committing the backend.
- S05 assumed `settings.rs` would exist when it ran (for `cycle_slug` refresh in Settings save path). In practice S04 had not been committed, so S05's Settings refresh step was a no-op. The Settings save path correctly calls `cycle_status` after saving in the final implementation.
- The S05 summary incorrectly stated "S04 landed Screen::Settings" — Screen::Settings was implemented during milestone close, not in S04's slice execution.
- `Config` import: early planning assumed `assay_core::config::Config`; actual location is `assay_types::Config`. Resolved in S01 and carried forward.

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added `[[bin]]` section, `assay-types.workspace = true`, `[lib]` section, `tempfile` dev-dependency
- `crates/assay-tui/src/lib.rs` — new; `pub mod wizard; pub mod wizard_draw;`
- `crates/assay-tui/src/main.rs` — rewritten as thin entry point; match on Key/Resize events
- `crates/assay-tui/src/app.rs` — new (was main.rs); all screens, App struct, draw/handle_event, all free render fns including draw_settings
- `crates/assay-tui/src/wizard.rs` — new; WizardState, StepKind, WizardAction, handle_wizard_event, draw_wizard
- `crates/assay-tui/tests/app_wizard.rs` — App-level wizard wiring tests
- `crates/assay-tui/tests/help_status.rs` — new; 6 contract tests for show_help toggle and cycle_slug loading
- `crates/assay-tui/tests/settings.rs` — new; 5 integration tests for Settings screen (open/close/nav/save/restart)
- `crates/assay-tui/tests/spec_browser.rs` — new; 6 integration tests for MilestoneDetail/ChunkDetail navigation
- `crates/assay-tui/tests/wizard_round_trip.rs` — new; integration test driving synthetic KeyEvents → create_from_inputs → file assertions
- `crates/assay-types/src/lib.rs` — ProviderKind enum, ProviderConfig struct, Config.provider field (D092)
- `crates/assay-types/tests/schema_snapshots.rs` — provider_kind_schema_snapshot and provider_config_schema_snapshot tests added
- `crates/assay-types/tests/schema_roundtrip.rs` — ProviderConfig round-trip test added
- `crates/assay-types/tests/snapshots/schema_snapshots__provider-kind-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__provider-config-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap` — updated snapshot (provider field added)
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap` — updated snapshot (mesh/gossip fields)
- `crates/assay-core/src/config/mod.rs` — `pub fn save(root, config)` added (D093)
- `crates/assay-core/tests/config_provider.rs` — new; 5 round-trip and save integration tests
- `crates/assay-mcp/src/server.rs` — Config initializations updated with `provider: None`
- `Cargo.lock` — aws-lc-rs and rustls-webpki bumped to clear RUSTSEC advisories
