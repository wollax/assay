---
estimated_steps: 7
estimated_files: 6
---

# T01: Add `ProviderKind`/`ProviderConfig` to `assay-types` and lock schema snapshot

**Slice:** S04 — Provider Configuration Screen
**Milestone:** M006

## Description

Add `ProviderKind` enum and `ProviderConfig` struct to `assay-types`, extend `Config` with an optional `provider` field following the D092/D056 pattern, write backward-compatibility roundtrip tests, and lock the updated `Config` schema snapshot. This is the type contract that all subsequent work in this slice depends on.

The critical constraint is `Config`'s `deny_unknown_fields` — the new `provider` field MUST have `#[serde(default, skip_serializing_if = "Option::is_none")]` or existing `config.toml` files without the `[provider]` section will fail to deserialize.

## Steps

1. Open `crates/assay-types/src/lib.rs`. Add `ProviderKind` enum just before the `Config` struct:
   ```rust
   /// AI provider backend selection.
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
   #[serde(rename_all = "snake_case")]
   pub enum ProviderKind {
       /// Anthropic Claude via the Claude Code adapter.
       #[default]
       Anthropic,
       /// OpenAI GPT via the OpenCode adapter.
       OpenAI,
       /// Ollama local inference.
       Ollama,
   }
   inventory::submit! {
       schema_registry::SchemaEntry {
           name: "provider-kind",
           generate: || schemars::schema_for!(ProviderKind),
       }
   }
   ```

2. Add `ProviderConfig` struct after `ProviderKind`:
   ```rust
   /// AI provider and model configuration.
   ///
   /// All fields are optional; omitted fields leave model selection to the adapter default.
   #[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
   #[serde(deny_unknown_fields)]
   pub struct ProviderConfig {
       /// Selected AI provider. Defaults to Anthropic.
       #[serde(default)]
       pub provider: ProviderKind,
       /// Model override for the planning phase.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub planning_model: Option<String>,
       /// Model override for the execution phase.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub execution_model: Option<String>,
       /// Model override for the review phase.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       pub review_model: Option<String>,
   }
   inventory::submit! {
       schema_registry::SchemaEntry {
           name: "provider-config",
           generate: || schemars::schema_for!(ProviderConfig),
       }
   }
   ```

3. Add `provider: Option<ProviderConfig>` to the `Config` struct with the required serde attrs:
   ```rust
   /// AI provider and model configuration.
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub provider: Option<ProviderConfig>,
   ```
   Add a doc-comment `/// AI provider configuration.` to match the code style.

4. Re-export `ProviderKind` and `ProviderConfig` in the `pub use` section of `lib.rs` alongside the other type re-exports.

5. Write `crates/assay-core/tests/config_provider.rs` with two roundtrip tests:
   - `config_toml_roundtrip_without_provider`: Create a tempdir, write `.assay/config.toml` with only `project_name = "test-proj"` (no `[provider]` section), call `assay_core::config::load(dir.path())` — assert it returns `Ok(config)` with `config.provider.is_none()`.
   - `config_toml_roundtrip_with_provider`: Create a tempdir, write `.assay/config.toml` with `project_name = "test-proj"\n[provider]\nprovider = "open_ai"\nplanning_model = "gpt-4o"`, call `config::load` — assert `config.provider` is `Some(ProviderConfig { provider: ProviderKind::OpenAI, planning_model: Some("gpt-4o"), .. })`.

6. Run `cargo test -p assay-types config_schema_snapshot` — it will fail with a snapshot diff. Run `cargo insta review` to accept the updated `Config` schema. Re-run to confirm green.

7. Add snapshot tests for the new types to `crates/assay-types/tests/schema_snapshots.rs`:
   ```rust
   #[test]
   fn provider_kind_schema_snapshot() {
       let schema = schemars::schema_for!(assay_types::ProviderKind);
       assert_json_snapshot!("provider-kind-schema", schema.to_value());
   }
   #[test]
   fn provider_config_schema_snapshot() {
       let schema = schemars::schema_for!(assay_types::ProviderConfig);
       assert_json_snapshot!("provider-config-schema", schema.to_value());
   }
   ```
   Run `cargo insta review` to accept the new snapshots.

## Must-Haves

- [ ] `ProviderKind` enum with `Anthropic` (default), `OpenAI`, `Ollama` — `serde(rename_all = "snake_case")` so `"open_ai"` round-trips correctly
- [ ] `ProviderConfig` struct with four fields all using `serde(default, skip_serializing_if = "Option::is_none")` for Option fields
- [ ] `Config.provider: Option<ProviderConfig>` with `serde(default, skip_serializing_if = "Option::is_none")` — NOT missing the `default` attr or existing configs break
- [ ] Both types have `inventory::submit!` schema entries
- [ ] `config_toml_roundtrip_without_provider` test passes — proves backward compat
- [ ] `config_toml_roundtrip_with_provider` test passes — proves forward compat
- [ ] `config_schema_snapshot` updated and accepted; `provider-kind-schema` and `provider-config-schema` snapshots created

## Verification

- `cargo test -p assay-types` — all pass including new snapshot tests
- `cargo test -p assay-core config_provider` — both roundtrip tests pass
- `cargo insta review` — shows no pending reviews after accepting new/updated snapshots
- `cargo test --workspace` — no regressions

## Observability Impact

- Signals added/changed: `Config.provider` field — `None` means no provider configured (user gets adapter default); `Some(ProviderConfig)` means explicit selection
- How a future agent inspects this: `assay_core::config::load(root)?.provider` — inspect the loaded config
- Failure state exposed: deserialization failure of `deny_unknown_fields` Config is now caught at load time with a `ConfigParse` error; backward-compat test proves this does NOT happen for existing configs

## Inputs

- `crates/assay-types/src/lib.rs` — existing `Config` struct (must be extended without breaking `deny_unknown_fields`)
- `crates/assay-types/tests/schema_snapshots.rs` — existing snapshot test file (add two new tests)
- D092 — mandates exactly the D056 pattern: `serde(default)` + `serde(skip_serializing_if)` for all new optional fields
- D056 — established the `..Default::default()` + `#[serde(default)]` pattern for backward-compatible struct extension

## Expected Output

- `crates/assay-types/src/lib.rs` — extended with `ProviderKind`, `ProviderConfig`, `Config.provider` field
- `crates/assay-core/tests/config_provider.rs` — new file with two passing roundtrip tests
- `crates/assay-types/tests/schema_snapshots.rs` — two new snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap` — updated (provider field in schema)
- `crates/assay-types/tests/snapshots/schema_snapshots__provider-kind-schema.snap` — new
- `crates/assay-types/tests/snapshots/schema_snapshots__provider-config-schema.snap` — new
