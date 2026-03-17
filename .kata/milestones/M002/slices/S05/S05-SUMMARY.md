---
id: S05
parent: M002
milestone: M002
provides:
  - "assay harness generate|install|update|diff CLI subcommands for claude-code, codex, opencode"
  - "check_scope() globset-based scope enforcement returning Vec<ScopeViolation>"
  - "generate_scope_prompt() multi-agent awareness markdown for PromptLayer injection"
  - "ScopeViolationType/ScopeViolation types in assay-types with schema snapshots"
  - "file_scope and shared_files fields on ManifestSession (backward compatible)"
requires:
  - slice: S04
    provides: "Codex and OpenCode adapter generate_config/write_config functions"
  - slice: S02
    provides: "Session context for multi-agent prompt generation (consumed indirectly via ManifestSession)"
affects:
  - S06
key_files:
  - crates/assay-types/src/harness.rs
  - crates/assay-types/src/manifest.rs
  - crates/assay-harness/src/scope.rs
  - crates/assay-cli/src/commands/harness.rs
key_decisions:
  - "D036: Scope types in assay-types, logic in assay-harness (harness-layer concern, not core)"
  - "D037: Scope prompt injected as PromptLayer (priority -100), not by modifying adapter signatures"
  - "D038: harness update overwrites all managed files (terraform/helm convention)"
patterns_established:
  - "GeneratedConfig enum wraps adapter-specific config with unified files()/write() interface for CLI dispatch"
  - "inject_scope_layer() adds PromptLayerKind::System prompt before adapter dispatch — adapters stay pure"
  - "GlobSet compiled once per check_scope invocation; pattern attribution via individual Glob matchers"
  - "SharedFileConflict takes priority over in-scope when a file matches both scope and shared_files"
observability_surfaces:
  - "assay harness diff <adapter> — exit code 0 (no changes) or 1 (changes detected), prints added/changed/removed to stderr"
  - "assay harness generate/install — prints file count and paths to stderr"
  - "check_scope() returns Vec<ScopeViolation> with file path, violation_type, and triggering pattern"
  - "Unknown adapter errors include valid adapter list for self-correction"
drill_down_paths:
  - .kata/milestones/M002/slices/S05/tasks/T01-SUMMARY.md
  - .kata/milestones/M002/slices/S05/tasks/T02-SUMMARY.md
  - .kata/milestones/M002/slices/S05/tasks/T03-SUMMARY.md
duration: 3 tasks
verification_result: passed
completed_at: 2026-03-17
---

# S05: Harness CLI & Scope Enforcement

**Full `assay harness generate|install|update|diff` CLI surface for all three adapters with globset-based scope enforcement and multi-agent awareness prompt injection.**

## What Happened

T01 added foundation types: `ScopeViolationType` enum (OutOfScope, SharedFileConflict) and `ScopeViolation` struct in assay-types with full serde derives, deny_unknown_fields, and inventory registration. Added `file_scope` and `shared_files` fields to `ManifestSession` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` for backward compatibility. Locked 4 schema snapshots (2 new + 2 updated). Updated all ManifestSession struct literals across the workspace.

T02 implemented the scope enforcement engine in `assay-harness::scope`: `check_scope()` builds a `GlobSet` from file_scope patterns and classifies changed files as OutOfScope or SharedFileConflict (shared takes priority when both match). `generate_scope_prompt()` produces multi-agent awareness markdown listing scope boundaries, shared files, and neighboring sessions with overlapping patterns. Added globset as a workspace dependency.

T03 built the CLI surface: `HarnessCommand` clap enum with Generate, Install, Update, and Diff sub-subcommands. Generate dispatches to all three adapters via a `GeneratedConfig` enum wrapper. Install/Update write config to project root (update is an alias). Diff compares generated config against existing files on disk and reports added/changed/removed with exit code 0/1. Scope prompt injection happens before adapter dispatch via `inject_scope_layer()` adding a PromptLayer with priority -100.

## Verification

- `cargo test -p assay-types` — 40 tests pass (scope types, schema snapshots, manifest round-trips)
- `cargo test -p assay-harness` — 58 tests pass (9 scope tests + 49 existing adapter tests)
- `cargo test -p assay-cli -- harness` — 11 tests pass (adapter validation, generation for all 3 adapters, diff detection, scope injection, install, file discovery)
- `just ready` — all checks pass (fmt, lint, test, deny)

## Requirements Advanced

- R022 (Harness orchestration layer) — scope enforcement and multi-agent prompt generation fully implemented; CLI dispatch surfaces complete

## Requirements Validated

- R022 — check_scope() detects violations via globset patterns; generate_scope_prompt() produces multi-agent awareness markdown; harness CLI generates/installs/diffs for all three adapters; 22 new tests (9 scope + 11 CLI + 2 type) lock the contract

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

None.

## Known Limitations

- `harness update` is a full regeneration (no incremental patch) — D038 documents this as intentional
- Scope enforcement is advisory only — `check_scope()` returns violations but nothing blocks agent execution on violations (blocking deferred to future work)
- No snapshot tests locking `generate_scope_prompt()` output format — prompt content tested by assertion but not snapshot-pinned

## Follow-ups

- S06 wires scope-aware config generation into orchestrated runs (scope prompt injection during parallel execution)
- S06 adds MCP tools for orchestration status and live session state

## Files Created/Modified

- `Cargo.toml` — added globset workspace dependency
- `crates/assay-types/src/harness.rs` — ScopeViolationType, ScopeViolation with derives and tests
- `crates/assay-types/src/manifest.rs` — file_scope, shared_files fields on ManifestSession
- `crates/assay-types/src/lib.rs` — re-exported scope types
- `crates/assay-types/tests/schema_snapshots.rs` — 2 new snapshot tests
- `crates/assay-types/tests/snapshots/` — 2 new + 2 updated snapshot files
- `crates/assay-harness/Cargo.toml` — globset dependency
- `crates/assay-harness/src/lib.rs` — pub mod scope
- `crates/assay-harness/src/scope.rs` — check_scope(), generate_scope_prompt(), 9 unit tests
- `crates/assay-cli/src/commands/harness.rs` — HarnessCommand, generate/install/update/diff, GeneratedConfig, 11 tests
- `crates/assay-cli/src/commands/mod.rs` — pub mod harness
- `crates/assay-cli/src/main.rs` — Harness variant in Command enum
- `crates/assay-core/src/manifest.rs` — updated ManifestSession literals
- `crates/assay-core/src/pipeline.rs` — updated ManifestSession literals
- `crates/assay-core/src/orchestrate/dag.rs` — updated ManifestSession literals
- `crates/assay-core/src/orchestrate/executor.rs` — updated ManifestSession literals
- `crates/assay-types/tests/schema_roundtrip.rs` — updated ManifestSession literals

## Forward Intelligence

### What the next slice should know
- The harness CLI is fully wired: `assay harness generate|install|update|diff` dispatches to all three adapters. S06 can call `assay harness generate` from integration tests or invoke the adapter functions directly.
- Scope prompt injection is a call-site concern — `inject_scope_layer()` in harness.rs adds the layer before dispatching. The orchestrator should do the same when generating per-session configs.

### What's fragile
- `GeneratedConfig` enum in harness.rs matches on adapter name strings ("claude-code", "codex", "opencode") — adding a new adapter requires updating the match arms and `VALID_ADAPTERS` const.
- `find_existing_adapter_files()` hardcodes known file paths per adapter — if adapter file layouts change, diff/removal detection breaks silently.

### Authoritative diagnostics
- `cargo test -p assay-cli -- harness` — 11 tests cover all CLI dispatch paths; failures here indicate adapter wiring issues
- `cargo test -p assay-harness -- scope` — 9 tests cover globset pattern matching edge cases

### What assumptions changed
- No assumptions changed. D027 (globset-based advisory scope) implemented as designed. All three adapters dispatched cleanly through unified GeneratedConfig enum.
