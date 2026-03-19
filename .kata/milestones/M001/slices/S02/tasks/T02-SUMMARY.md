---
id: T02
parent: S02
milestone: M001
provides:
  - assay-harness crate scaffolded in workspace with module stubs (prompt, settings, claude)
  - 6 schema snapshot tests for all harness types (HarnessProfile, PromptLayer, PromptLayerKind, SettingsOverride, HookContract, HookEvent)
  - assay-harness workspace dependency entry in root Cargo.toml
key_files:
  - crates/assay-harness/Cargo.toml
  - crates/assay-harness/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
key_decisions: []
patterns_established:
  - Harness crate stub modules follow doc-comment-only pattern for future S03/S04 implementation
observability_surfaces:
  - Schema snapshot .snap files in crates/assay-types/tests/snapshots/ provide deterministic JSON schema for all harness types
  - cargo build -p assay-harness validates dependency graph
  - cargo insta test -p assay-types shows diffs for any schema drift
duration: 5m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T02: Scaffold assay-harness crate, add schema snapshots, pass just ready

**Created `assay-harness` crate with module stubs, added 6 schema snapshot tests for all harness types, and verified full suite passes with `just ready`.**

## What Happened

1. Added `assay-harness = { path = "crates/assay-harness" }` to root workspace dependencies.
2. Created `crates/assay-harness/Cargo.toml` with workspace deps on `assay-core`, `assay-types`, `serde`, `serde_json`.
3. Created `crates/assay-harness/src/lib.rs` with `#![deny(missing_docs)]`, crate doc comment, and three module stubs: `prompt`, `settings`, `claude`.
4. Created stub module files (`prompt.rs`, `settings.rs`, `claude.rs`) each with a doc comment describing their future purpose.
5. Added 6 schema snapshot tests to `crates/assay-types/tests/schema_snapshots.rs` following the existing pattern.
6. Ran `cargo insta test -p assay-types --accept` — all 6 new snapshots generated and accepted.
7. Ran `just ready` — all checks passed (fmt, lint, 32 snapshot tests, 629 core tests, 91 MCP tests, deny).

## Verification

- `cargo build -p assay-harness` — compiled successfully
- `cargo insta test -p assay-types --accept` — 6 new snapshots accepted, 32 total snapshot tests pass
- `just ready` — all checks passed (fmt-check, clippy, test, deny)
- Slice-level checks:
  - ✅ `cargo build -p assay-harness` — crate compiles with correct dependency edges
  - ✅ `cargo insta test -p assay-types` — no pending snapshots
  - ✅ `cargo test -p assay-types -- schema_snapshots` — all 32 snapshot tests pass
  - ✅ `just ready` — full suite passes
  - ✅ `rg "HarnessProfile" crates/assay-types/src/lib.rs` — type is re-exported (from T01)
  - ✅ `rg "deny_unknown_fields" crates/assay-types/src/harness.rs` — every struct has the attribute (from T01)

## Diagnostics

- Schema snapshot `.snap` files in `crates/assay-types/tests/snapshots/` — 6 new files for harness types
- `cargo insta test -p assay-types` detects schema drift with diffs
- `cargo build -p assay-harness` validates workspace dependency graph

## Deviations

- `cargo fmt` required a line-break fix in `assay-types/src/lib.rs` re-export line (auto-formatted, no manual intervention needed)

## Known Issues

None.

## Files Created/Modified

- `Cargo.toml` — added `assay-harness` workspace dependency
- `crates/assay-harness/Cargo.toml` — new crate manifest
- `crates/assay-harness/src/lib.rs` — crate root with `#![deny(missing_docs)]` and 3 module stubs
- `crates/assay-harness/src/prompt.rs` — stub module for prompt builder (S03)
- `crates/assay-harness/src/settings.rs` — stub module for settings merger (S03)
- `crates/assay-harness/src/claude.rs` — stub module for Claude adapter (S04)
- `crates/assay-types/tests/schema_snapshots.rs` — 6 new snapshot test functions
- `crates/assay-types/tests/snapshots/schema_snapshots__harness-profile-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__prompt-layer-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__prompt-layer-kind-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__settings-override-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__hook-contract-schema.snap` — new snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__hook-event-schema.snap` — new snapshot
