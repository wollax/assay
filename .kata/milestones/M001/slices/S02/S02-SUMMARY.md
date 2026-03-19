---
id: S02
parent: M001
milestone: M001
provides:
  - assay-harness crate in workspace with module stubs (prompt, settings, claude)
  - HarnessProfile type system in assay-types (6 types with full derives, schema registry, deny_unknown_fields)
  - 6 schema snapshot tests locking the harness type contract
  - assay-harness workspace dependency entry in root Cargo.toml
requires:
  - slice: S01
    provides: Clean compilation foundation with GateEvalContext rename complete
affects:
  - S03
  - S04
  - S06
key_files:
  - crates/assay-types/src/harness.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-harness/Cargo.toml
  - crates/assay-harness/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
key_decisions: []
patterns_established:
  - Harness types follow same derive/deny_unknown_fields/inventory::submit! pattern as existing types
  - Harness crate stub modules use doc-comment-only pattern for future implementation
observability_surfaces:
  - Schema snapshot .snap files in crates/assay-types/tests/snapshots/ provide deterministic JSON schemas for all harness types
  - cargo build -p assay-harness validates workspace dependency graph
  - cargo insta test -p assay-types detects schema drift with diffs
drill_down_paths:
  - .kata/milestones/M001/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S02/tasks/T02-SUMMARY.md
duration: 10m
verification_result: passed
completed_at: 2026-03-16
---

# S02: Harness Crate & Profile Type

**`assay-harness` crate scaffolded in workspace with `HarnessProfile` type system (6 types) locked by schema snapshots.**

## What Happened

T01 created the 6 harness configuration types in `assay-types/src/harness.rs`: `HarnessProfile`, `PromptLayer`, `PromptLayerKind`, `SettingsOverride`, `HookContract`, and `HookEvent`. All types have full derives (`Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema`), `deny_unknown_fields` on every struct, `inventory::submit!` with kebab-case names, and doc comments on every type and field. Types re-exported from `lib.rs`.

T02 scaffolded `crates/assay-harness/` as a workspace leaf crate depending on `assay-core` and `assay-types`, with `#![deny(missing_docs)]` and three module stubs (`prompt`, `settings`, `claude`) for S03/S04 to fill. Added 6 schema snapshot tests and the workspace dependency entry. All 32 snapshot tests pass, `just ready` passes.

## Verification

- `cargo build -p assay-harness` — compiles with correct dependency edges
- `cargo insta test -p assay-types` — no pending snapshots (32 total pass)
- `cargo test -p assay-types -- schema_snapshots` — all snapshot tests pass
- `just ready` — full suite passes (fmt, clippy, test, deny)
- `rg "HarnessProfile" crates/assay-types/src/lib.rs` — type re-exported
- `rg "deny_unknown_fields" crates/assay-types/src/harness.rs` — 4 occurrences (one per struct)

## Requirements Advanced

- R003 (Harness crate exists) — `assay-harness` crate exists as a workspace leaf with correct dependency edges, compiles successfully
- R004 (HarnessProfile type) — Full type system defined with 6 types, schema snapshots locked, re-exported from assay-types

## Requirements Validated

- R003 — Crate compiles, workspace dependency graph verified by `cargo build -p assay-harness`
- R004 — Type contract locked by schema snapshot tests; all derives, deny_unknown_fields, and inventory registration verified

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

None.

## Known Limitations

- Module stubs in assay-harness are empty (doc comments only) — S03 and S04 fill them
- HarnessProfile fields are designed for the Claude Code adapter; additional harness adapters (M003) may need extensions

## Follow-ups

- none

## Files Created/Modified

- `crates/assay-types/src/harness.rs` — 6 harness configuration types with full derives and schema registry
- `crates/assay-types/src/lib.rs` — added `pub mod harness` and re-exports
- `crates/assay-harness/Cargo.toml` — new crate manifest
- `crates/assay-harness/src/lib.rs` — crate root with `#![deny(missing_docs)]` and 3 module stubs
- `crates/assay-harness/src/prompt.rs` — stub for S03
- `crates/assay-harness/src/settings.rs` — stub for S03
- `crates/assay-harness/src/claude.rs` — stub for S04
- `crates/assay-types/tests/schema_snapshots.rs` — 6 new snapshot tests
- `crates/assay-types/tests/snapshots/` — 6 new .snap files for harness types
- `Cargo.toml` — added `assay-harness` workspace dependency

## Forward Intelligence

### What the next slice should know
- All 6 harness types are in `assay-types/src/harness.rs` and re-exported from `assay_types`. Import as `assay_types::HarnessProfile`, etc.
- Schema snapshots exist for all types — any field changes will cause `cargo insta test` to show pending diffs. Run `cargo insta test --accept` after intentional changes.
- The `PromptLayer` struct has `kind: PromptLayerKind`, `name: String`, `content: String`, `priority: i32` — S03's prompt builder consumes these.
- `SettingsOverride` has `model`, `permissions`, `tools`, `max_turns` — all `Option<T>` with `skip_serializing_if = "Option::is_none"`.
- `HookContract` has `event: HookEvent`, `command: String`, `timeout_secs: Option<u64>`.

### What's fragile
- Nothing fragile — these are compile-time types with snapshot tests locking the contract

### Authoritative diagnostics
- `cargo insta test -p assay-types` — shows diffs for any schema drift; this is the primary signal for type contract changes
- `cargo build -p assay-harness` — validates the dependency graph is correct

### What assumptions changed
- No assumptions changed — execution matched the plan exactly
