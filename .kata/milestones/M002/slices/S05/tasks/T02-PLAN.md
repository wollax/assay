---
estimated_steps: 5
estimated_files: 4
---

# T02: Implement scope enforcement and multi-agent prompt generation in assay-harness

**Slice:** S05 — Harness CLI & Scope Enforcement
**Milestone:** M002

## Description

Create `scope.rs` in assay-harness with two core functions: `check_scope()` uses globset to match file paths against `file_scope` and `shared_files` patterns, returning `Vec<ScopeViolation>` for any violations (advisory, not blocking per D027). `generate_scope_prompt()` produces a concise multi-agent awareness markdown string listing the session's scope boundaries and neighboring sessions, suitable for injection as a `PromptLayer` with `PromptLayerKind::System` and priority -100.

## Steps

1. Add `globset` as a workspace dependency in root `Cargo.toml` and add it to `crates/assay-harness/Cargo.toml`.
2. Create `crates/assay-harness/src/scope.rs` with `check_scope(file_scope: &[String], shared_files: &[String], changed_files: &[String]) -> Vec<ScopeViolation>`. Build a `GlobSet` from `file_scope` patterns; for each changed file, check if it matches file_scope OR shared_files. If it matches neither and file_scope is non-empty, it's `OutOfScope`. If it matches shared_files, it's `SharedFileConflict` (advisory warning). Empty `file_scope` means no restrictions — return empty vec.
3. Add `generate_scope_prompt(session_name: &str, file_scope: &[String], shared_files: &[String], all_sessions: &[(String, Vec<String>, Vec<String>)]) -> String` that produces multi-agent awareness markdown. List this session's owned scope, shared files, and direct neighbors (other sessions whose scopes overlap or share files). Keep output concise — only neighbors, not all sessions.
4. Add `pub mod scope;` to `crates/assay-harness/src/lib.rs`.
5. Write unit tests covering: (a) empty file_scope returns no violations, (b) file matching file_scope returns no violations, (c) file outside file_scope returns OutOfScope, (d) file matching shared_files returns SharedFileConflict, (e) glob patterns with `**/*.rs` and `{src,tests}/**` work, (f) generate_scope_prompt produces expected markdown, (g) generate_scope_prompt with no neighbors is concise.

## Must-Haves

- [ ] `globset` workspace dependency added
- [ ] `check_scope()` uses GlobSet compiled once per invocation
- [ ] Empty file_scope means no restrictions (return empty vec)
- [ ] OutOfScope violations carry the file path and best-match pattern context
- [ ] SharedFileConflict violations are advisory (not blocking)
- [ ] `generate_scope_prompt()` produces concise markdown with scope boundaries and neighbors
- [ ] At least 7 unit tests covering enforcement edge cases and prompt generation

## Verification

- `cargo test -p assay-harness -- scope` — all scope tests pass
- `cargo test -p assay-harness` — no regression in existing 49 tests
- `cargo clippy -p assay-harness` — no warnings

## Observability Impact

- Signals added/changed: `check_scope()` returns structured `ScopeViolation` vec — callers can inspect count, types, and specific files
- How a future agent inspects this: call `check_scope()` with changed file list after agent execution; non-empty result means scope boundary crossed
- Failure state exposed: `ScopeViolation.violation_type` distinguishes `OutOfScope` (hard boundary) from `SharedFileConflict` (coordination needed)

## Inputs

- `crates/assay-types/src/harness.rs` — ScopeViolation and ScopeViolationType types from T01
- `crates/assay-harness/src/prompt.rs` — build_prompt() pattern for reference
- S05-RESEARCH.md — globset API: GlobSetBuilder::new(), add(), build(), matches()

## Expected Output

- `Cargo.toml` — globset in workspace dependencies
- `crates/assay-harness/Cargo.toml` — globset dependency added
- `crates/assay-harness/src/scope.rs` — check_scope() + generate_scope_prompt() with ~7 unit tests
- `crates/assay-harness/src/lib.rs` — pub mod scope added
