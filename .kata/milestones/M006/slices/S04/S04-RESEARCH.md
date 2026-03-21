# S04: Provider Configuration Screen — Research

**Date:** 2026-03-21
**Confidence:** HIGH

## Summary

S04 adds provider configuration to the TUI: a `ProviderKind`/`ProviderConfig` type pair in `assay-types`, a `config_save` write function in `assay-core`, and a `Screen::Settings` UI in `assay-tui`. The most critical risk is the `deny_unknown_fields` on `Config` — any mistake in the backward-compat serde annotations breaks existing `config.toml` files silently. The schema snapshot update is mechanical but required.

**Important deviation from S01-SUMMARY:** The actual current `App` struct does NOT have `config: Option<Config>` or `show_help: bool` fields, and `Screen` does NOT have `MilestoneDetail`, `ChunkDetail`, or `Settings` variants. S01 deviated from its stated plan and only delivered the four variants needed for S01+S02 (`NoProject`, `Dashboard`, `Wizard(WizardState)`, `LoadError(String)`). S04 must add `App.config`, `Screen::Settings(SettingsState)`, and wire `s` key handling — these are not pre-existing stubs.

The implementation is three tasks: (T01) extend `assay-types` with provider types + schema lock, (T02) add `config_save` to `assay-core::config`, (T03) add `SettingsState` + `Screen::Settings` + `App.config` field and tests to `assay-tui`. All three tasks are straightforward given the established patterns.

## Recommendation

Follow the D056/D092 pattern exactly for the types layer: `#[serde(rename_all = "snake_case")]` on `ProviderKind` with an explicit `#[serde(rename = "open_ai")]` on the `OpenAI` variant (consecutive uppercase letters in serde snake_case conversion are ambiguous — use explicit rename to guarantee `"open_ai"` TOML output). For the TUI Settings screen, keep it simple: a 3-item provider selection list (no text-input model fields in the first pass) — the plan specifies model fields as "optional model hints" and the S04 `must-haves` focus on provider persistence; model fields can render as static hint text or be deferred to a follow-on.

Use `toml::to_string_pretty` for `config_save` (the same function used in `wizard.rs:378` — established precedent over `toml::to_string` used in `milestone_save`). This produces more readable output for a user-facing config file.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic config write | `NamedTempFile::new_in + write_all + flush + sync_all + persist` (verbatim from `milestone_save`, lines 130-148 in `crates/assay-core/src/milestone/mod.rs`) | Battle-tested pattern; `.new_in` puts the temp file in the same directory so rename is atomic across filesystems |
| TOML serialization | `toml::to_string_pretty(&config)` (used in `wizard.rs:378`) | Produces human-readable output; already in workspace deps via `crates/assay-core` |
| Schema snapshot test | `insta::assert_json_snapshot!` (all 40+ snapshots in `crates/assay-types/tests/schema_snapshots.rs`) | One-line test; locked by `cargo insta review`; all existing types use this pattern |
| Enum serde snake_case | `#[serde(rename_all = "snake_case")]` on enum (used by `OrchestratorMode`, `MilestoneStatus`, and 10+ other enums) | Consistent with entire codebase; results in `"anthropic"`, `"open_ai"`, `"ollama"` in TOML |
| Borrow-checker for stateful widget | D097 pattern: pass individual fields (`Option<&Config>`, `&mut SettingsState`) to `draw_settings`, not `&mut App` | Same pattern used by `draw_dashboard(frame, milestones, list_state)` — avoids dual-borrow panic with `render_stateful_widget` |
| List selection wrapping | Current `handle_event` Dashboard arm: `(s + 1).min(len - 1)` / `s.saturating_sub(1)` | Note: this actually uses clamping not wrapping — follow the same clamping pattern for the 3-item provider list; wrapping is not required for a 3-item menu |

## Existing Code and Patterns

- `crates/assay-types/src/lib.rs` (lines 174–225) — `Config` struct with `#[serde(deny_unknown_fields)]`. Five existing optional subsections (`gates`, `guard`, `worktree`, `sessions`) all use `#[serde(default, skip_serializing_if = "Option::is_none")]`. The new `provider` field must follow this exact pattern.
- `crates/assay-types/src/lib.rs` (lines 1–58) — module imports and `pub use` re-exports. New `ProviderKind` and `ProviderConfig` must be re-exported here (not just declared in `lib.rs` inline; or if inline, add to the re-export list at the top).
- `crates/assay-core/src/milestone/mod.rs` (lines 118–150) — `milestone_save`: the exact NamedTempFile pattern. Copy verbatim for `config_save`, substituting the path (`root.join(".assay").join("config.toml")`).
- `crates/assay-core/src/wizard.rs` (line 378) — `toml::to_string_pretty(&spec)` — use the same function for `config_save` (milestone_save uses `toml::to_string` which omits pretty-printing; `wizard.rs` uses `to_string_pretty`; prefer pretty for user-facing config).
- `crates/assay-types/src/milestone.rs` (lines 1–30) — `MilestoneStatus` enum with `#[derive(Default)]`, `#[default]` attribute, and `#[serde(rename_all = "snake_case")]`. This is the exact pattern for `ProviderKind`.
- `crates/assay-types/tests/schema_snapshots.rs` (line 28) — `config_schema_snapshot` test, `assert_json_snapshot!("config-schema", schema.to_value())`. This snapshot WILL fail after adding `provider` to `Config` — run `cargo insta review` to accept.
- `crates/assay-tui/src/app.rs` — Current `App` struct has `screen`, `milestones`, `list_state`, `project_root` (no `config` field). The `with_project_root` constructor must be extended to call `assay_core::config::load(root).ok()` and store the result in the new `config` field.
- `crates/assay-tui/src/wizard.rs` — `WizardState` struct and `draw_wizard`/`handle_wizard_event` functions. Follow this module structure for `settings.rs`: `SettingsState` struct + `draw_settings` + `handle_settings_event` (or inline in `app.rs`).
- `crates/assay-tui/tests/app_wizard.rs` — Integration test pattern: construct `App::with_project_root(Some(tempdir))`, drive via `handle_event(key(...))`, assert on `app.screen` and `app.config`. Follow for `tests/settings_screen.rs`.

## Constraints

- `Config` has `#[serde(deny_unknown_fields)]`. **Every new field on `Config` requires `#[serde(default, skip_serializing_if = "Option::is_none")]`** — missing `serde(default)` causes `toml::from_str` to fail when the field is absent from an existing `config.toml`.
- `ProviderConfig` itself should NOT use `deny_unknown_fields` — the sub-struct is new and users may add comments; `deny_unknown_fields` on the parent is sufficient.
- `assay-core` is sync (D007). `config_save` must be a plain `std::fs` write with `NamedTempFile` — no async.
- `toml::to_string_pretty` wraps the error as `toml::ser::Error`; map it to `AssayError::Io` with `InvalidData` kind (same pattern as `milestone_save` lines 124–129).
- The `assay-tui` binary name derives from `[package] name = "assay-tui"` in `Cargo.toml` — no `[[bin]]` section is needed (Cargo auto-produces `assay-tui` binary from `src/main.rs` when the crate name has a hyphen). This is already confirmed working (`target/debug/assay-tui` exists).
- `config_save` must be publicly exported from `assay-core::config` (not just `pub(crate)`) — `assay-tui` calls `assay_core::config::config_save(root, &config)`.
- Schema snapshots are locked by insta. After adding `provider` to `Config`, `cargo test -p assay-types` will fail until `cargo insta review` is run to accept the updated snapshot. This is expected and required.

## Common Pitfalls

- **`serde(default)` is required, not just `skip_serializing_if`** — forgetting `#[serde(default)]` on `Config.provider` means any `config.toml` without a `[provider]` section fails to parse. `skip_serializing_if = "Option::is_none"` only affects serialization, not deserialization. Both annotations are needed.
- **`OpenAI` snake_case ambiguity** — serde's `rename_all = "snake_case"` on `OpenAI` may produce `"open_a_i"` depending on the serde version. Use explicit `#[serde(rename = "open_ai")]` on the `OpenAI` variant to guarantee the expected TOML string.
- **`App.config` field not present** — the S01-SUMMARY claims `App.config` was added in S01, but the actual `app.rs` has no such field. T03 must add `config: Option<assay_types::Config>` to `App` and update `with_project_root` to load it.
- **`Screen::Settings` not present** — same as above: the `Screen` enum currently has 4 variants; S04 adds the 5th (`Settings(SettingsState)`).
- **`draw()` match arms must be exhaustive** — when `Screen::Settings(...)` is added, the `draw()` method's `match &self.screen` must include an arm for it. The current match has 4 arms; adding a 5th variant without a draw arm causes a compile error (which is good, but easy to miss if adding the variant in one file and forgetting to update `draw()`).
- **`config_save` error: `NamedTempFile::new_in` needs the `.assay/` directory to exist** — if `.assay/` doesn't exist (e.g., first-time init), `new_in` will fail. Guard with `std::fs::create_dir_all(&assay_dir)` before the NamedTempFile call. The `config::load` path already assumes `.assay/` exists (it reads `config.toml` inside it), but `config_save` might be called to write a new config to a newly-init'd project.
- **`toml::to_string` vs `to_string_pretty`** — `milestone_save` uses `to_string`; `wizard.rs` uses `to_string_pretty`. Use `to_string_pretty` for `config_save` to produce human-readable output the user might inspect.
- **`ProviderConfig` needs `deny_unknown_fields` too** — without it, a user who adds an unsupported field to `[provider]` in their `config.toml` will silently get that field ignored instead of an error. Consider adding `#[serde(deny_unknown_fields)]` to `ProviderConfig` for consistency with all other `Config` sub-structs (`GatesConfig`, `GuardConfig`, `SessionsConfig`, `WorktreeConfig` all have it).

## Open Risks

- **`ProviderKind` schema snapshot count**: Adding `ProviderKind` and `ProviderConfig` to `assay-types` requires two new `inventory::submit!` blocks and two new snapshot tests. Each new test adds to the snapshot count — `cargo insta review` must be run after T01. Not a risk, just a process reminder.
- **Model field UX complexity**: The plan mentions "model selection per phase" (planning_model, execution_model, review_model). Text input inside the TUI requires the same cursor-management complexity as the wizard. If the must-haves require model text input, follow the wizard's `WizardState.fields` pattern. If model fields are optional for the first pass (and can be edited directly in `config.toml`), defer to a follow-on to reduce T03 scope.
- **`config::load` on Settings open vs startup**: Currently `main.rs` does not call `config::load` — the `App` struct has no `config` field. T03 adds the field and loads config in `with_project_root`. If the project has no `config.toml` (valid state), `config::load` returns `Err(AssayError::Io { ... "No such file" })` — use `.ok()` to convert to `None` (not `unwrap_or_default`, which would invent a config with empty `project_name` that fails validation). A `None` config means "no provider configured yet" — Settings screen shows Anthropic as default.
- **`config_save` overwrites the entire config**: `config_save` serializes the whole `Config` struct. If `App.config` is `None` (no existing config), the Settings screen cannot save provider settings without a `project_name` to populate. Plan: if `App.config` is `None`, construct a minimal `Config { project_name: "assay-project", specs_dir: "specs/", ..Default::default() }` as a fallback, or refuse to save (show "Cannot save: no config loaded") and document this as a known limitation.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | (searched in S03 research) | No dedicated skill found |

## Sources

- `crates/assay-types/src/lib.rs` — `Config` struct, existing optional sub-config fields pattern (HIGH confidence)
- `crates/assay-types/src/milestone.rs` — `MilestoneStatus` enum with `rename_all = "snake_case"` + `#[default]` attribute (HIGH confidence)
- `crates/assay-core/src/milestone/mod.rs` — `milestone_save()` NamedTempFile atomic write pattern (HIGH confidence)
- `crates/assay-core/src/wizard.rs` — `toml::to_string_pretty` usage (HIGH confidence)
- `crates/assay-types/tests/schema_snapshots.rs` — `config_schema_snapshot` test, `assert_json_snapshot!` pattern (HIGH confidence)
- `crates/assay-tui/src/app.rs` — current `App` struct fields (4, not 6 as S01-SUMMARY stated); current `Screen` enum variants (4, not 6) (HIGH confidence — read directly from source)
- `git log --oneline` — confirms S01 and S02 merged; S03 NOT merged; current codebase is post-S02 state (HIGH confidence)
