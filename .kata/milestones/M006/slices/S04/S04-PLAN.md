# S04: Provider Configuration Screen

**Goal:** Add `ProviderConfig` to `assay-types` (backward-compatible), `config_save` to `assay-core`, and a fully functional Settings screen in `assay-tui` that lists AI providers (Anthropic, OpenAI, Ollama), allows selection and model entry, and persists the choice to `.assay/config.toml` via atomic write. Existing `config.toml` files without a provider section load without error. Schema snapshot locked.

**Demo:** Press `s` from the Dashboard → Settings screen opens showing the current provider selection (or Anthropic as default). Arrow keys move through three providers; optional model fields can be edited. Press `w` (write) to save → `.assay/config.toml` updated atomically → pressing `s` again shows the persisted selection. An existing `config.toml` without the `[provider]` section round-trips without error.

## Must-Haves

- `ProviderKind` enum (`Anthropic | OpenAI | Ollama`) and `ProviderConfig` struct in `assay-types` — all fields `serde(default, skip_serializing_if)` per D092
- `Config.provider: Option<ProviderConfig>` with `serde(default, skip_serializing_if = "Option::is_none")` — backward-compatible with existing `deny_unknown_fields`
- Schema snapshot for `Config` updated and locked via `cargo insta review`
- `config_toml_roundtrip_without_provider` test: existing `config.toml` without `[provider]` loads without error
- `config_toml_roundtrip_with_provider` test: `ProviderConfig` serializes and deserializes correctly through TOML
- `config_save(root: &Path, config: &Config) -> Result<()>` in `assay-core::config` using NamedTempFile + sync_all + persist (D093)
- `config: Option<Config>` field on `App` struct in `assay-tui`, loaded from `config::load` in `main.rs`
- `Screen::Settings(SettingsState)` variant in the `Screen` enum
- `draw_settings(frame, config, settings_state)` free function — lists three providers, highlights selected, shows model hints
- `s` key from Dashboard opens Settings; `Esc` / `q` returns to Dashboard without saving
- `w` key in Settings saves config via `config_save` and returns to Dashboard
- Tests in `tests/settings_screen.rs`: open-settings transition, Esc-returns-to-Dashboard, save updates config, no-project graceful-degradation

## Proof Level

- This slice proves: contract + integration
- Real runtime required: no (tests use tempdir; no real terminal needed)
- Human/UAT required: yes (visual Settings UI verification, provider persistence across restart)

## Verification

- `cargo test -p assay-types` — new `config_provider_roundtrip_*` tests pass; schema snapshot locked
- `cargo test -p assay-core` — new `config_save_*` tests pass
- `cargo test -p assay-tui` — new `tests/settings_screen.rs` tests pass; total ≥ 27 tests
- `cargo test --workspace` — no regressions (≥ 1356 prior tests)
- `just ready` — fmt, lint, test, deny all green

## Observability / Diagnostics

- Runtime signals: `App.config` field — inspect to know whether config was loaded from disk; `None` means no `.assay/config.toml` found (graceful degradation)
- Inspection surfaces: `.assay/config.toml` on disk after save; `cargo test -p assay-tui settings` to exercise save path; `App.screen` variant — `Screen::Settings(_)` confirms settings is open
- Failure visibility: `config_save` returns `AssayError::Io` with path and operation label on failure; Settings screen renders inline error on save failure (same pattern as wizard's `state.error`)
- Redaction constraints: no secrets in config (API keys are env-level, not in `config.toml`)

## Integration Closure

- Upstream surfaces consumed: `assay_types::Config` (extended with `provider`), `assay_core::config::load` (already called in `main.rs`), `App.project_root` (available in S01)
- New wiring introduced in this slice: `config_save` in `assay-core::config`; `App.config` field; `Screen::Settings` variant and `draw_settings` render fn; `s` key handler in `App::handle_event`
- What remains before the milestone is truly usable end-to-end: S03 (chunk detail view), S05 (help overlay + status bar + `just ready` final pass)

## Tasks

- [ ] **T01: Add `ProviderKind`/`ProviderConfig` to `assay-types` and lock schema snapshot** `est:45m`
  - Why: The type foundation — backward-compatible Config extension following D092 (D056 pattern). Tests and snapshot lock the contract before TUI wiring.
  - Files: `crates/assay-types/src/lib.rs`, `crates/assay-core/tests/config_provider.rs` (new), `crates/assay-types/tests/schema_snapshots.rs`, `crates/assay-types/tests/snapshots/` (updated)
  - Do: (1) Add `ProviderKind` enum (`Anthropic | OpenAI | Ollama`, derives: Debug/Clone/Copy/PartialEq/Eq/Serialize/Deserialize/JsonSchema, default=Anthropic) and `ProviderConfig` struct (`provider: ProviderKind`, `planning_model: Option<String>`, `execution_model: Option<String>`, `review_model: Option<String>` — all `serde(default, skip_serializing_if = "Option::is_none")`) to `crates/assay-types/src/lib.rs`. (2) Add `provider: Option<ProviderConfig>` to `Config` with `serde(default, skip_serializing_if = "Option::is_none")`. (3) Add inventory schema entries for `ProviderKind` and `ProviderConfig`. (4) Re-export `ProviderKind` and `ProviderConfig` from `assay-types`. (5) Write `crates/assay-core/tests/config_provider.rs` with `config_toml_roundtrip_without_provider` (load a config.toml with no `[provider]` section — must succeed) and `config_toml_roundtrip_with_provider` (write a `config.toml` with `[provider] provider = "open_ai"` and check it round-trips). (6) Run `cargo test -p assay-core config_provider` — tests must fail initially if schema snapshot is outdated. (7) Update schema snapshot: `cargo test -p assay-types config_schema_snapshot` will fail with a diff; run `cargo insta review` to accept the new schema. Re-run tests to confirm green.
  - Verify: `cargo test -p assay-types` and `cargo test -p assay-core config_provider` — all pass; `cargo insta review` shows no pending reviews
  - Done when: Both roundtrip tests pass; schema snapshot for `Config` updated and committed; `cargo test -p assay-types` fully green

- [ ] **T02: Add `config_save` to `assay-core::config` and test atomic write** `est:30m`
  - Why: The TUI must not write config files directly — it goes through `assay-core` (D093). `config_save` follows the established NamedTempFile pattern from `milestone_save`.
  - Files: `crates/assay-core/src/config/mod.rs`, `crates/assay-core/tests/config_provider.rs` (extend T01 test file)
  - Do: (1) Add `pub fn config_save(root: &Path, config: &Config) -> Result<()>` to `crates/assay-core/src/config/mod.rs`. Pattern: `let assay_dir = root.join(".assay"); let path = assay_dir.join("config.toml"); let content = toml::to_string_pretty(config)?; let mut tmpfile = NamedTempFile::new_in(&assay_dir)?; tmpfile.write_all(content.as_bytes())?; tmpfile.as_file().sync_all()?; tmpfile.persist(&path)?`. Handle errors with `AssayError::Io`. (2) Export `config_save` from the module's public API. (3) Add `config_save_creates_file`, `config_save_overwrites_existing`, and `config_save_with_provider_persists` tests to `crates/assay-core/tests/config_provider.rs`. The `config_save_with_provider_persists` test: write config with `ProviderConfig { provider: OpenAI, planning_model: Some("gpt-4") }` → `config_save` → `config::load` → assert provider and model field match.
  - Verify: `cargo test -p assay-core config_provider` — all new tests pass
  - Done when: `config_save` compiles, is exported, and all three new tests pass; `cargo test -p assay-core` fully green

- [ ] **T03: Add `SettingsState`, `Screen::Settings`, and Settings screen to `assay-tui`** `est:1h`
  - Why: The user-facing deliverable. Wires the TUI settings screen to the type and persistence infrastructure from T01/T02.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/settings.rs` (new), `crates/assay-tui/src/lib.rs`, `crates/assay-tui/tests/settings_screen.rs` (new), `crates/assay-tui/Cargo.toml`
  - Do: (1) Add `SettingsState { list_state: ListState, error: Option<String> }` struct in `crates/assay-tui/src/settings.rs`; add `draw_settings(frame: &mut Frame, config: Option<&Config>, state: &mut SettingsState)` free function that renders a bordered list of three provider options (Anthropic, OpenAI, Ollama) with the current provider highlighted and a hint line (`↑↓ select · w save · Esc cancel`). (2) Add `pub mod settings;` to `lib.rs`. (3) Add `config: Option<Config>` field to `App` struct in `app.rs`. Update `with_project_root` to call `assay_core::config::load(root).ok()` and store result. (4) Add `Screen::Settings(SettingsState)` variant to the `Screen` enum. (5) Add `s` key dispatch in `handle_event` Dashboard branch: open `Screen::Settings(SettingsState::new())`. (6) Add Settings arm to `handle_event`: `Esc`/`q` → `Screen::Dashboard`; `Up`/`Down` → move list selection with wrapping; `w` → read current list selection → build updated `ProviderConfig` → update `self.config` → call `config_save` if `project_root` is Some; on error set `state.error`; on success return to `Screen::Dashboard`. (7) Add Settings arm to `draw()`. (8) Write `tests/settings_screen.rs` with: `settings_opens_from_dashboard` (press `s` → Screen::Settings); `settings_esc_returns_to_dashboard` (press `Esc` → Screen::Dashboard; config unchanged); `settings_save_updates_app_config` (with tempdir project, press `s`, navigate, press `w` → App::config updated and disk file written with correct provider); `settings_save_no_project_no_crash` (App::new() with None root → no panic on `w`). Note: follow D097 borrow-checker pattern — pass `Option<&Config>` and `&mut SettingsState` as separate parameters to `draw_settings`, not `&mut App`.
  - Verify: `cargo test -p assay-tui settings` — 4 tests pass; `cargo test -p assay-tui` — total ≥ 27 tests
  - Done when: All 4 new settings tests pass; `s` key opens Settings; `w` saves to disk; `Esc` cancels without saving; `cargo test --workspace` fully green; `just ready` passes

## Files Likely Touched

- `crates/assay-types/src/lib.rs` — add ProviderKind, ProviderConfig, Config.provider field, schema entries, re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — add provider_kind_schema_snapshot and provider_config_schema_snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap` — updated snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__provider-kind-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__provider-config-schema.snap` — new snapshot
- `crates/assay-core/src/config/mod.rs` — add config_save function
- `crates/assay-core/tests/config_provider.rs` — new test file (T01+T02 tests)
- `crates/assay-tui/src/lib.rs` — add pub mod settings
- `crates/assay-tui/src/app.rs` — add config field, Screen::Settings variant, s/w key handlers, draw arm
- `crates/assay-tui/src/settings.rs` — new: SettingsState, draw_settings
- `crates/assay-tui/tests/settings_screen.rs` — new: 4 tests
