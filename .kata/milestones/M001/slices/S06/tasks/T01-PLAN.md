---
estimated_steps: 5
estimated_files: 3
---

# T01: Define RunManifest and ManifestSession types with schema snapshots

**Slice:** S06 — RunManifest Type & Parsing
**Milestone:** M001

## Description

Create the `RunManifest` and `ManifestSession` types in `assay-types/src/manifest.rs` following the established type patterns (harness.rs, config types). Add schema snapshot tests to lock the contract. These types are the input contract for the S07 pipeline — the manifest is what users author to declare work.

`ManifestSession` must not embed `HarnessProfile` directly (research pitfall) — instead it contains the relevant harness override fields (`settings`, `hooks`, `prompt_layers`) as optional inline overrides plus a required `spec` field. The pipeline (S07) constructs a full `HarnessProfile` from manifest + spec + defaults.

## Steps

1. Create `crates/assay-types/src/manifest.rs` with `RunManifest` and `ManifestSession` types:
   - `RunManifest`: top-level struct with `sessions: Vec<ManifestSession>` (maps to `[[sessions]]` TOML)
   - `ManifestSession`: `spec: String` (required), `name: Option<String>`, `settings: Option<SettingsOverride>`, `hooks: Vec<HookContract>`, `prompt_layers: Vec<PromptLayer>`
   - Full derives: `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema`
   - `deny_unknown_fields` on both structs
   - `inventory::submit!` with kebab-case names (`run-manifest`, `manifest-session`)
   - Doc comments on every type and field
2. Add `pub mod manifest` to `crates/assay-types/src/lib.rs` and re-export `RunManifest`, `ManifestSession`
3. Add `run_manifest_schema_snapshot()` and `manifest_session_schema_snapshot()` to `crates/assay-types/tests/schema_snapshots.rs`
4. Run `cargo insta test -p assay-types --accept` to generate and accept snapshot files
5. Verify `cargo build -p assay-core` still compiles (no breakage from new module)

## Must-Haves

- [ ] `RunManifest` and `ManifestSession` types compile with full derives
- [ ] `deny_unknown_fields` on both structs
- [ ] `inventory::submit!` registered for both types
- [ ] Types re-exported from `assay-types/src/lib.rs`
- [ ] Schema snapshot tests pass for both types
- [ ] `[[sessions]]` TOML array format supported (Vec<ManifestSession>)
- [ ] No trait methods — types are plain structs (D001)

## Verification

- `cargo test -p assay-types -- schema_snapshots::run_manifest_schema_snapshot` passes
- `cargo test -p assay-types -- schema_snapshots::manifest_session_schema_snapshot` passes
- `cargo build -p assay-core` compiles without errors

## Observability Impact

- Signals added/changed: Schema snapshot `.snap` files for `run-manifest-schema` and `manifest-session-schema` detect future type drift
- How a future agent inspects this: `cargo insta test -p assay-types` shows diffs for any schema changes
- Failure state exposed: None (compile-time types only)

## Inputs

- `crates/assay-types/src/harness.rs` — `SettingsOverride`, `HookContract`, `PromptLayer` types used as optional overrides in `ManifestSession`
- `crates/assay-types/src/lib.rs` — re-export pattern to follow
- `crates/assay-types/tests/schema_snapshots.rs` — snapshot test pattern to follow
- S06 research — `ManifestSession` name (avoids collision with `SessionEntry` in context.rs)

## Expected Output

- `crates/assay-types/src/manifest.rs` — 2 types with full derives, doc comments, schema registry
- `crates/assay-types/src/lib.rs` — updated with `pub mod manifest` and re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — 2 new snapshot tests
- `crates/assay-types/tests/snapshots/run-manifest-schema.snap` — locked schema
- `crates/assay-types/tests/snapshots/manifest-session-schema.snap` — locked schema
