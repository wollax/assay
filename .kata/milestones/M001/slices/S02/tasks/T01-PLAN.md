---
estimated_steps: 4
estimated_files: 2
---

# T01: Define HarnessProfile type system in assay-types

**Slice:** S02 — Harness Crate & Profile Type
**Milestone:** M001

## Description

Create the `HarnessProfile` type and all its sub-types (`PromptLayer`, `PromptLayerKind`, `SettingsOverride`, `HookContract`, `HookEvent`) in `assay-types/src/harness.rs`. These types define the complete agent configuration contract consumed by S03 (prompt builder, settings merger), S04 (Claude adapter), and S06 (RunManifest). All types follow the codebase's standard derive pattern, use `deny_unknown_fields`, register with the schema registry, and are re-exported from `lib.rs`.

## Steps

1. Create `crates/assay-types/src/harness.rs` with module-level doc comment and all types:
   - `PromptLayerKind` enum: `System`, `Project`, `Spec`, `Custom` variants (kebab-case serde rename). Represents the category/priority of a prompt layer.
   - `PromptLayer` struct: `kind: PromptLayerKind`, `name: String`, `content: String`, `priority: i32` (ordering hint for assembly). `deny_unknown_fields`.
   - `SettingsOverride` struct: `model: Option<String>`, `permissions: Vec<String>`, `tools: Vec<String>`, `max_turns: Option<u32>`. `deny_unknown_fields`, skip_serializing_if on Options/Vecs.
   - `HookEvent` enum: `PreTool`, `PostTool`, `Stop` variants (kebab-case). Lifecycle events the harness can hook into.
   - `HookContract` struct: `event: HookEvent`, `command: String`, `timeout_secs: Option<u64>`. `deny_unknown_fields`.
   - `HarnessProfile` struct: `name: String`, `prompt_layers: Vec<PromptLayer>`, `settings: SettingsOverride`, `hooks: Vec<HookContract>`, `working_dir: Option<String>`. `deny_unknown_fields`.
   - Each type gets: `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema` derives, doc comments on the type and every public field.
   - Each type gets `inventory::submit!` with kebab-case name (e.g., `"harness-profile"`, `"prompt-layer"`, `"hook-contract"`, etc.)

2. Add `pub mod harness;` to `crates/assay-types/src/lib.rs` (alphabetical order with other modules).

3. Add `pub use` re-exports to `crates/assay-types/src/lib.rs`:
   ```
   pub use harness::{HarnessProfile, HookContract, HookEvent, PromptLayer, PromptLayerKind, SettingsOverride};
   ```

4. Verify: `cargo build -p assay-types` compiles clean with no warnings.

## Must-Haves

- [ ] `HarnessProfile` struct with `prompt_layers`, `settings`, `hooks`, `working_dir` fields
- [ ] `PromptLayer` struct with `kind`, `name`, `content`, `priority` fields
- [ ] `PromptLayerKind` enum with `System`, `Project`, `Spec`, `Custom` variants
- [ ] `SettingsOverride` struct with `model`, `permissions`, `tools`, `max_turns` fields
- [ ] `HookContract` struct with `event`, `command`, `timeout_secs` fields
- [ ] `HookEvent` enum with `PreTool`, `PostTool`, `Stop` variants
- [ ] `#[serde(deny_unknown_fields)]` on every struct
- [ ] `#[serde(rename_all = "kebab-case")]` on every enum
- [ ] `inventory::submit!` for every type
- [ ] Doc comments on every public type and field (`#![deny(missing_docs)]` in assay-types)
- [ ] Re-exports in `lib.rs`

## Verification

- `cargo build -p assay-types` compiles with no warnings
- `rg "HarnessProfile" crates/assay-types/src/lib.rs` shows the re-export
- `rg "deny_unknown_fields" crates/assay-types/src/harness.rs` shows one per struct (4 structs)
- `rg "inventory::submit" crates/assay-types/src/harness.rs` shows 6 entries

## Observability Impact

- Signals added/changed: None (compile-time types only)
- How a future agent inspects this: `rg` for type names across the codebase; `cargo doc -p assay-types` generates browsable docs
- Failure state exposed: Compiler errors if derives are wrong or fields are missing

## Inputs

- `crates/assay-types/src/session.rs` — pattern for derives, `deny_unknown_fields`, `inventory::submit!`
- `crates/assay-types/src/feature_spec.rs` — pattern for nested struct composition with optional fields
- `crates/assay-types/src/work_session.rs` — pattern for lifecycle enums with `rename_all = "kebab-case"`
- S02 research recommendations for field design

## Expected Output

- `crates/assay-types/src/harness.rs` — new file with 6 types, all derives, all registrations
- `crates/assay-types/src/lib.rs` — updated with `mod harness` and `pub use` re-exports
