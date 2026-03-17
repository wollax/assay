# S02: Harness Crate & Profile Type — UAT

**Milestone:** M001
**Written:** 2026-03-16

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: This slice produces compile-time types and a crate scaffold with no runtime behavior. Schema snapshots lock the type contract deterministically. No live runtime or human experience testing is needed.

## Preconditions

- Rust toolchain installed with `cargo` available
- `cargo-insta` installed (`cargo install cargo-insta`)
- Repository checked out with S01 complete on the current branch

## Smoke Test

`cargo build -p assay-harness && cargo insta test -p assay-types` — both succeed with no errors or pending snapshots.

## Test Cases

### 1. Harness crate compiles with correct dependencies

1. Run `cargo build -p assay-harness`
2. **Expected:** Compilation succeeds. Output shows `assay-types` and `assay-core` as transitive dependencies.

### 2. All harness types are re-exported

1. Run `rg "HarnessProfile|PromptLayer|SettingsOverride|HookContract|HookEvent" crates/assay-types/src/lib.rs`
2. **Expected:** All 6 type names appear in the `pub use` re-export line.

### 3. Schema snapshots are deterministic

1. Run `cargo insta test -p assay-types`
2. **Expected:** All 32 snapshot tests pass, no pending snapshots. Output says "no snapshots to review".

### 4. deny_unknown_fields on all structs

1. Run `rg "deny_unknown_fields" crates/assay-types/src/harness.rs`
2. **Expected:** 4 occurrences (one per struct: HarnessProfile, PromptLayer, SettingsOverride, HookContract).

### 5. Full suite passes

1. Run `just ready`
2. **Expected:** All checks pass (fmt, clippy, test, deny).

## Edge Cases

### Schema drift detection

1. Temporarily add a field to `HarnessProfile` in `harness.rs`
2. Run `cargo insta test -p assay-types`
3. **Expected:** Pending snapshot diff shown for `harness-profile-schema`. Test fails until snapshot is accepted.
4. Revert the change.

## Failure Signals

- `cargo build -p assay-harness` fails — dependency graph is broken
- `cargo insta test` shows pending snapshots — schema has drifted from accepted snapshots
- `just ready` fails on clippy — missing doc comments or unused imports
- `rg "deny_unknown_fields"` returns fewer than 4 matches — a struct is missing the attribute

## Requirements Proved By This UAT

- R003 (Harness crate exists) — Test case 1 proves the crate exists and compiles with correct dependency edges
- R004 (HarnessProfile type) — Test cases 2, 3, 4 prove the type system is complete with schema contract, strict deserialization, and public re-exports

## Not Proven By This UAT

- Runtime behavior of harness types (no runtime code exists yet — S03/S04 scope)
- Prompt building, settings merging, or hook translation (S03 scope)
- Claude Code config generation from HarnessProfile (S04 scope)
- Integration with RunManifest (S06 scope)

## Notes for Tester

This is a compile-time-only slice. If all test cases pass, the contract is locked. The module stubs in `assay-harness/src/` are intentionally empty — they exist only to establish the crate structure for S03/S04.
