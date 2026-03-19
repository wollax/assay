---
id: T02
parent: S05
milestone: M002
provides:
  - check_scope() function for file-access violation detection via globset
  - generate_scope_prompt() function for multi-agent awareness markdown
key_files:
  - crates/assay-harness/src/scope.rs
  - crates/assay-harness/Cargo.toml
  - Cargo.toml
key_decisions:
  - SharedFileConflict takes priority over in-scope when a file matches both scope and shared_files
  - Neighbor detection uses exact pattern string matching between sessions (sufficient for manifest-declared patterns)
patterns_established:
  - GlobSet compiled once per check_scope invocation; individual Glob matchers used only for pattern attribution
observability_surfaces:
  - check_scope() returns Vec<ScopeViolation> with file path, violation_type, and triggering pattern — callers inspect count and types
  - generate_scope_prompt() output is a plain markdown string suitable for PromptLayer injection
duration: 10min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Implement scope enforcement and multi-agent prompt generation in assay-harness

**Added `check_scope()` and `generate_scope_prompt()` in `assay-harness::scope` with globset-based enforcement and 9 unit tests.**

## What Happened

Created `crates/assay-harness/src/scope.rs` with two public functions:

1. `check_scope(file_scope, shared_files, changed_files) -> Vec<ScopeViolation>` — builds a `GlobSet` from `file_scope` patterns and checks each changed file. Empty `file_scope` means no restrictions. Files matching `shared_files` get `SharedFileConflict`; files matching neither get `OutOfScope`. SharedFileConflict takes priority when a file matches both.

2. `generate_scope_prompt(session_name, file_scope, shared_files, all_sessions) -> String` — produces concise markdown listing the session's scope, shared files, and neighbors (other sessions with overlapping patterns). No-neighbor case omits the section entirely.

Added `globset = "0.4"` as a workspace dependency and wired it into `assay-harness/Cargo.toml`.

## Verification

- `cargo test -p assay-harness -- scope` — 9 tests pass (7 required + 2 additional edge cases)
- `cargo test -p assay-harness` — 58 tests pass, no regression (was 49 before scope tests)
- `cargo clippy -p assay-harness` — clean, no warnings
- `cargo test -p assay-types -- scope` — 2 scope type tests pass (T01 artifacts)
- `cargo test -p assay-types -- schema_snapshots` — schema snapshot tests pass

## Diagnostics

- `check_scope()` returns structured `ScopeViolation` values — inspect `.violation_type` for `OutOfScope` vs `SharedFileConflict`, `.file` for the path, `.pattern` for the triggering glob
- `generate_scope_prompt()` returns plain markdown — print or inject as a `PromptLayer` with `PromptLayerKind::System` and priority -100

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `Cargo.toml` — added `globset = "0.4"` to workspace dependencies
- `crates/assay-harness/Cargo.toml` — added `globset.workspace = true`
- `crates/assay-harness/src/scope.rs` — new: check_scope() + generate_scope_prompt() with 9 unit tests
- `crates/assay-harness/src/lib.rs` — added `pub mod scope;`
