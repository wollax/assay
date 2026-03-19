---
id: T01
parent: S02
milestone: M001
provides:
  - HarnessProfile type system in assay-types (6 types: HarnessProfile, PromptLayer, PromptLayerKind, SettingsOverride, HookContract, HookEvent)
  - Re-exports from assay-types lib.rs
key_files:
  - crates/assay-types/src/harness.rs
  - crates/assay-types/src/lib.rs
key_decisions: []
patterns_established:
  - Harness types follow same derive/deny_unknown_fields/inventory::submit! pattern as existing types
observability_surfaces:
  - none (compile-time types only)
duration: 5m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T01: Define HarnessProfile type system in assay-types

**Created 6 harness configuration types in `assay-types/src/harness.rs` with full derives, schema registry, and re-exports.**

## What Happened

Created `crates/assay-types/src/harness.rs` with all 6 types per plan:
- `PromptLayerKind` enum (System/Project/Spec/Custom, kebab-case)
- `PromptLayer` struct (kind/name/content/priority, deny_unknown_fields)
- `SettingsOverride` struct (model/permissions/tools/max_turns, deny_unknown_fields, skip_serializing_if on optionals)
- `HookEvent` enum (PreTool/PostTool/Stop, kebab-case)
- `HookContract` struct (event/command/timeout_secs, deny_unknown_fields)
- `HarnessProfile` struct (name/prompt_layers/settings/hooks/working_dir, deny_unknown_fields)

All types have `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema` derives, doc comments on every type and field, and `inventory::submit!` with kebab-case names. Added `pub mod harness` and `pub use` re-exports to `lib.rs`.

## Verification

- `cargo build -p assay-types` — compiles clean, zero warnings
- `cargo test -p assay-types` — all 26 existing tests pass
- `rg "HarnessProfile" crates/assay-types/src/lib.rs` — re-export confirmed
- `rg "deny_unknown_fields" crates/assay-types/src/harness.rs` — 4 occurrences (one per struct)
- `rg "inventory::submit" crates/assay-types/src/harness.rs` — 6 entries

## Slice-level verification (partial — T01 is intermediate):

- `cargo build -p assay-harness` — N/A (crate doesn't exist yet, T02 scope)
- `cargo insta test -p assay-types` — no pending snapshots (new snapshot tests are T02 scope)
- `cargo test -p assay-types -- schema_snapshots` — 26 existing pass, new tests are T02 scope
- `just ready` — not run (T02 will run after crate scaffold)
- `rg "HarnessProfile" crates/assay-types/src/lib.rs` — ✅ PASS
- `rg "deny_unknown_fields" crates/assay-types/src/harness.rs` — ✅ PASS

## Diagnostics

Compile-time types only. Inspect via `rg` for type names or `cargo doc -p assay-types`.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/harness.rs` — new file with 6 harness configuration types
- `crates/assay-types/src/lib.rs` — added `pub mod harness` and `pub use` re-exports
