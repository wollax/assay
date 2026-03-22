---
id: S04
parent: M006
milestone: M006
provides:
  - ProviderKind enum (Anthropic|OpenAi|Ollama) in assay-types with serde(rename_all = "snake_case"), Default = Anthropic, schema snapshot locked
  - ProviderConfig struct in assay-types with serde(deny_unknown_fields): provider: ProviderKind, planning_model/execution_model/review_model: Option<String> all with serde(default, skip_serializing_if)
  - Config.provider: Option<ProviderConfig> field with serde(default, skip_serializing_if = "Option::is_none") — backward-compat with deny_unknown_fields
  - config_save (pub fn save) in assay-core::config using NamedTempFile+write_all+sync_all+persist pattern (D093)
  - schema_snapshots__provider-kind-schema.snap and schema_snapshots__provider-config-schema.snap locked
  - schema_snapshots__config-schema.snap updated (provider field added)
  - config_toml_roundtrip_without_provider test: existing config.toml without [provider] loads without error
  - config_toml_roundtrip_with_provider test: ProviderConfig serializes/deserializes correctly through TOML
  - config_save_creates_file, config_save_overwrites_existing, config_save_with_provider_persists tests (5 total in tests/config_provider.rs)
  - Screen::Settings { selected: usize, error: Option<String> } variant in assay-tui Screen enum
  - draw_settings free function: full-screen bordered block, provider list with ▶ highlight and [saved] marker, ↑↓ selection, w/Esc key hints, inline error
  - App.config: Option<assay_types::Config> (real Config type, loaded from config::load in with_project_root)
  - s key from Dashboard opens Settings; pre-selects current provider from App.config
  - ↑↓ navigation in Settings cycles among 3 providers (wrapping)
  - w key saves ProviderConfig to .assay/config.toml via config_save; refreshes cycle_slug; returns to Dashboard
  - Esc/q in Settings returns to Dashboard without saving
  - Inline error displayed when config_save fails (state.error pattern, D103)
  - 5 settings integration tests in tests/settings.rs (open/close/navigate/save/restart-persistence)
requires:
  - slice: S01
    provides: App struct, Screen enum, App.project_root, handle_event dispatch, draw() match dispatch
affects:
  - S05
key_files:
  - crates/assay-types/src/lib.rs
  - crates/assay-core/src/config/mod.rs
  - crates/assay-core/tests/config_provider.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__provider-kind-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__provider-config-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/settings.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - "D092 — ProviderConfig in assay-types follows D056 pattern exactly (serde default + skip_serializing_if + schema snapshot + backward-compat test)"
  - "D093 — config_save (save fn) in assay-core::config using NamedTempFile atomic write pattern"
  - "D101 — Settings screen 'w' key writes/saves; Esc/q cancel without saving"
  - "D102 — Settings screen is full-screen bordered block (not popup like wizard)"
  - "D103 — w save with no loaded config shows inline error; does not auto-create config.toml"
patterns_established:
  - "D092 backward-compat Config extension pattern: Option<T> field with serde(default, skip_serializing_if = 'Option::is_none') + schema snapshot update + round-trip test without the field"
  - "Settings full-screen pattern: bordered List of 3 providers with ▶ highlight and [saved] marker; key hints in bottom Paragraph"
  - "pre-select on open: 's' key derives selected index from App.config.provider before transitioning to Screen::Settings"
observability_surfaces:
  - "App.config.as_ref().and_then(|c| c.provider.as_ref()) — current persisted provider; None means no [provider] in config.toml"
  - "Screen::Settings { selected, error } — selected=0/1/2 for Anthropic/OpenAI/Ollama; error=Some(msg) on save failure"
  - ".assay/config.toml on disk — authoritative provider selection after w save"
  - "cargo test -p assay-tui --test settings — 5 tests serve as executable spec for all Settings transitions"
  - "cargo test -p assay-core --test config_provider — 5 tests serve as executable spec for TOML round-trip and atomic save"
duration: ~1.5h (backend types + tests: ~45min, Settings UI + settings.rs tests: ~45min, cargo insta accept: ~5min)
verification_result: passed
completed_at: 2026-03-21
---

# S04: Provider Configuration Screen

**ProviderKind/ProviderConfig types (backward-compat D092), config_save atomic write (D093), Screen::Settings full-screen provider selector, 5 settings integration tests including restart-persistence — R052 validated.**

## What Happened

S04 split across two phases of work. The backend types were developed first (as working directory changes not immediately committed), and the Settings screen UI was completed during the milestone-close step alongside committing all accumulated backend work.

**Backend (assay-types + assay-core):** Added `ProviderKind` enum (Anthropic/OpenAi/Ollama, snake_case serde, default=Anthropic) and `ProviderConfig` struct (provider + three optional model strings, all `serde(default, skip_serializing_if)`) to `assay-types/src/lib.rs`. Extended `Config.provider: Option<ProviderConfig>` with `serde(default, skip_serializing_if = "Option::is_none")` — the D056/D092 pattern that makes existing config.toml files load without error. Added `pub fn save(root, config)` to `assay-core::config` using NamedTempFile+sync_all+persist (consistent with milestone_save). Added schema registry entries for both new types; ran `cargo insta accept` to lock all three snapshots (provider_kind, provider_config, config with provider). Five integration tests in `tests/config_provider.rs` prove round-trip and save semantics.

**Settings UI (assay-tui):** Added `Screen::Settings { selected: usize, error: Option<String> }` to the Screen enum. Changed `App.config` from `Option<AppConfig>` (the S05 placeholder) to `Option<assay_types::Config>` and populated it from `assay_core::config::load` in `with_project_root`. Added `draw_settings` as a full-screen bordered block with a `List` of three provider options (Anthropic/OpenAI/Ollama) and a `Paragraph` key hint line. Added `s` key handler in Dashboard arm to open Settings with pre-selected provider, `↑↓` navigation, `w` save (calls `config_save`, refreshes `cycle_slug`, returns to Dashboard), and `Esc`/`q` cancel. Inline error pattern (same as wizard `state.error`) surfaces save failures. Five integration tests in `tests/settings.rs` prove the complete lifecycle.

## Verification

- `cargo test -p assay-core --test config_provider` → 5/5 pass
- `cargo test -p assay-types --test schema_snapshots` → 46/46 pass (all snapshots locked)
- `cargo test -p assay-tui --test settings` → 5/5 pass (open/close/navigate/save/restart-persistence)
- `cargo test --workspace` → 1367 tests, 0 failures
- `just ready` → fmt ✓, lint ✓, test ✓, deny ✓ — "All checks passed"

## Requirements Validated

- R052 — TUI provider configuration: Screen::Settings implemented; `s`→settings→`w`→save→Dashboard round-trip proven; provider survives TUI restart (config written to disk and re-read on App construction); backward-compat test proves existing config.toml without [provider] loads without error; schema snapshots locked

## Deviations

- Backend work accumulated in working directory without immediate commit — committed during milestone-close step alongside the Settings UI. No functional deviation; test coverage and schema snapshots were complete before the commit.
- S05 ran before S04's Settings UI was implemented. S05's `App.config` was `Option<AppConfig>` (a minimal local struct with only `project_name`). During S04 UI implementation, `App.config` was changed to `Option<assay_types::Config>` and all references updated. The S05 forward-intelligence note ("AppConfig struct — must be replaced with Config once S04 merges") was followed exactly.
- Model-per-phase fields (planning_model, execution_model, review_model) exist in `ProviderConfig` but the Settings UI shows only provider selection. Model editing requires manual `config.toml` editing. This matches the S04 plan's "must-haves" (provider selection + `w` save) — model text input was a nice-to-have not in the minimal scope.

## Known Limitations

- The Settings screen does not provide input fields for `planning_model`, `execution_model`, or `review_model`. These can be set manually in `.assay/config.toml`. A future iteration (M007 or M008) can add text input fields to the Settings screen.
- `w` save with no project config (e.g. no `.assay/config.toml`) creates a minimal `Config` with empty `project_name`. This is a known UX tradeoff per D103 — the user should run `assay init` to create a proper config.

## Files Created/Modified

- `crates/assay-types/src/lib.rs` — ProviderKind enum, ProviderConfig struct, Config.provider field (D092)
- `crates/assay-core/src/config/mod.rs` — `pub fn save(root, config)` added (D093)
- `crates/assay-core/tests/config_provider.rs` — new; 5 integration tests
- `crates/assay-types/tests/schema_snapshots.rs` — provider_kind and provider_config snapshot tests added
- `crates/assay-types/tests/schema_roundtrip.rs` — ProviderConfig round-trip test added
- `crates/assay-types/tests/snapshots/schema_snapshots__provider-kind-schema.snap` — new
- `crates/assay-types/tests/snapshots/schema_snapshots__provider-config-schema.snap` — new
- `crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap` — updated
- `crates/assay-mcp/src/server.rs` — Config struct literals updated with `provider: None`
- `crates/assay-tui/src/app.rs` — Screen::Settings variant; App.config type changed; draw_settings; s key; Settings handle_event arm
- `crates/assay-tui/tests/settings.rs` — new; 5 integration tests
