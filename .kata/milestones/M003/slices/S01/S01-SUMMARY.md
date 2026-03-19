---
id: S01
parent: M003
milestone: M003
provides:
  - Two-phase merge_execute() with abort_on_conflict parameter (leaves working tree conflicted for handler)
  - ConflictResolutionConfig type in assay-types with serde/schemars/inventory (schema snapshot locked)
  - resolve_conflict() sync function: reads conflicted files, builds structured prompt, spawns claude -p subprocess, parses JSON, stages and commits
  - conflict_resolution_enabled on MergeRunnerConfig with panic-safe handler invocation and automatic git merge --abort cleanup
  - CLI --conflict-resolution auto|skip flag on assay run composing the AI handler when auto
  - MCP orchestrate_run conflict_resolution parameter with same handler composition
requires: []
affects:
  - S02
key_files:
  - crates/assay-core/src/merge.rs
  - crates/assay-core/src/orchestrate/conflict_resolver.rs
  - crates/assay-core/src/orchestrate/merge_runner.rs
  - crates/assay-core/src/orchestrate/mod.rs
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - D043: Sync subprocess for conflict resolution (std::process::Command, not async run_evaluator)
  - D044: Two-phase merge lifecycle (abort_on_conflict parameter; default true preserves existing behavior)
  - D045: Scripted resolver for deterministic testing (real Claude is UAT only)
  - D046: ConflictResolutionOutput crate-local to assay-core (internal subprocess protocol, not persistence type)
  - D047: ConflictResolutionMode CLI enum crate-local to assay-cli (presentation concern)
  - D048: MCP conflict_resolution as Option<String> (matches failure_policy/merge_strategy pattern)
patterns_established:
  - Two-phase merge lifecycle: abort_on_conflict=false leaves working tree conflicted; handler resolves in-place; runner verifies SHA, falls back to merge --abort on failure
  - Sync subprocess with timeout: try_wait() polling at 100ms intervals; kill on timeout; collect stdout/stderr after exit
  - Claude envelope parsing reused from evaluator.rs (is_error check → structured_output extraction)
  - Panic-safe handler invocation: catch_unwind(AssertUnwindSafe) around conflict handler; panic → descriptive error + merge --abort
  - Conditional handler composition at call site (CLI/MCP): match on mode, compose appropriate closure; no Box<dyn Fn> overhead
observability_surfaces:
  - tracing::warn on subprocess failure (session_name, error, raw_output_len); tracing::info on success (session_name, sha, resolved_files count)
  - MergeSessionResult.error carries descriptive failure reason: "conflict handler panicked — merge aborted", "conflict handler returned invalid SHA: <sha>", "conflict skipped — conflicting files: <list>"
  - MergeReport.results distinguishes Merged (with valid merge_sha) from ConflictSkipped with error detail
  - assay run --help shows --conflict-resolution flag with valid values; invalid CLI/MCP values produce actionable error messages
drill_down_paths:
  - .kata/milestones/M003/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M003/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M003/slices/S01/tasks/T03-SUMMARY.md
  - .kata/milestones/M003/slices/S01/tasks/T04-SUMMARY.md
duration: ~70min (4 tasks × ~15-25min each)
verification_result: passed
completed_at: 2026-03-17
---

# S01: AI Conflict Resolution

**Two-phase `merge_execute()`, sync `resolve_conflict()` subprocess, conflict handler lifecycle in merge runner, and `--conflict-resolution auto` CLI/MCP surface — proven by integration tests with real git repos and a scripted resolver.**

## What Happened

**T01** laid the foundation by adding `abort_on_conflict: bool` to `merge_execute()`. When `false`, the auto-abort block is skipped and the working tree remains conflicted (conflict markers in files, `MERGE_HEAD` present), enabling a downstream handler to resolve in-place. All existing callers updated to pass `true`. `ConflictResolutionConfig` type added to `assay-types` (`enabled`, `model`, `timeout_secs`, defaults to `enabled: false` / `claude-sonnet-4-20250514` / 120s) with schema snapshot locked.

**T02** built the core AI resolver in a new `conflict_resolver.rs` module. `resolve_conflict()` spawns `claude -p --output-format json --json-schema <schema>` synchronously via `std::process::Command`, piping the conflict prompt via stdin. The prompt includes the full file contents (with markers), session name, file count, and conflict scan summary. Response parsed through the Claude envelope (`is_error` / `structured_output`), resolved file contents written to disk, staged with `git add`, committed with `git commit --no-edit` (which uses the MERGE_HEAD to produce a proper merge commit). Returns `ConflictAction::Resolved(sha)` on success or `ConflictAction::Skip` on any error. Timeout enforced via `try_wait()` polling at 100ms intervals. 18 unit tests cover all branches without real subprocess calls.

**T03** wired the two-phase lifecycle into `merge_runner.rs`. Added `conflict_resolution_enabled: bool` to `MergeRunnerConfig` (default `false`). When enabled, `merge_execute(..., abort_on_conflict: false)` is called; on conflict, the handler is invoked via `catch_unwind(AssertUnwindSafe)`. Successful resolution verified by `git rev-parse --verify`. Failures (Skip, Abort, panic, invalid SHA) all trigger `git merge --abort` to restore clean repo state. Two integration tests prove the lifecycle against a real git repo: one with a scripted resolver that strips markers, stages, and commits; one proving panic recovery and Skip fallback.

**T04** completed the slice by wiring into both user-facing entry points. CLI gets `--conflict-resolution auto|skip` (default `skip`) with a `ConflictResolutionMode` enum and clap value parser matching the `failure_policy`/`merge_strategy` pattern. When `auto`, a handler closure composing `resolve_conflict()` is passed to `merge_completed_sessions()` alongside `conflict_resolution_enabled: true`. MCP `OrchestrateRunParams` gains `conflict_resolution: Option<String>` parsed to a bool, with the same handler composition inside `spawn_blocking`. A pre-existing unused import warning in `conflict_resolver.rs` was fixed as part of getting `just ready` green.

## Verification

- `cargo test -p assay-core merge_execute_two_phase` — 2 passed
- `cargo test -p assay-core merge_runner_conflict_resolution` — 2 passed
- `cargo test -p assay-core resolve_conflict` — 1 passed (subprocess not-found graceful fallback)
- `cargo test -p assay-types schema_snapshots` — 53 passed (including ConflictResolutionConfig snapshot)
- `cargo test -p assay-cli run` — 16 passed (4 new conflict-resolution flag tests)
- `cargo test -p assay-mcp orchestrate_run` — 8 passed (3 new conflict_resolution param tests)
- `just ready` — fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓ — fully green

## Requirements Advanced

- R026 (AI conflict resolution) — all core infrastructure delivered: two-phase merge lifecycle, sync resolver subprocess, merge runner integration, CLI and MCP entry points

## Requirements Validated

- R026 — integration test proves: conflicting branches → handler called with live conflicted tree → scripted resolver strips markers, stages, commits → MergeReport shows Merged with valid merge commit SHA (2-parent history verified). CLI `--conflict-resolution auto` parses and routes to handler. MCP `conflict_resolution: "auto"` deserializes and routes. Validated at integration level; real Claude invocation is manual UAT.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T01 extracted `setup_conflicting_repo()` helper shared between two-phase tests and existing conflict test — minor refactor, improves test readability, not a plan deviation.
- T04 fixed a pre-existing unused import warning (`git_raw`) in `conflict_resolver.rs` left from T02 — required for `just lint` / `just ready` to pass. Not in T04 plan but a necessary prerequisite for slice completion.

## Known Limitations

- Real Claude invocation (`resolve_conflict()` with `claude` in PATH) is not exercised in automated tests — only in UAT. The subprocess path is tested only for the not-found case (returns `Skip` gracefully). Integration test uses a scripted resolver.
- `ConflictResolutionOutput` is a crate-local type (D046). S02 will need to either re-expose this type or define a separate `ConflictResolution` audit record type for MergeReport.
- Post-resolution validation command is not yet implemented — S02 concern.
- Audit trail (original markers, resolved content, resolver stdout) is not recorded in MergeReport — S02 concern.

## Follow-ups

- S02: Add `ConflictResolution` audit record type to MergeReport, validation command support, orchestrate_status resolution details, and end-to-end CLI integration test through the full pipeline.

## Files Created/Modified

- `crates/assay-core/src/merge.rs` — Added `abort_on_conflict` parameter, gated abort block, 2 integration tests + shared helper
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — New module: resolve_conflict(), prompt builders, ConflictResolutionOutput type, schema cache, 18 unit tests
- `crates/assay-core/src/orchestrate/merge_runner.rs` — conflict_resolution_enabled config, two-phase path, catch_unwind + SHA verify + abort cleanup, 2 integration tests
- `crates/assay-core/src/orchestrate/mod.rs` — Added `pub mod conflict_resolver;`
- `crates/assay-types/src/orchestrate.rs` — ConflictResolutionConfig type + 4 unit tests
- `crates/assay-types/src/lib.rs` — ConflictResolutionConfig re-export
- `crates/assay-types/tests/schema_snapshots.rs` — conflict_resolution_config_schema_snapshot test
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap` — New snapshot
- `crates/assay-cli/src/commands/run.rs` — ConflictResolutionMode enum + parse_conflict_resolution + --conflict-resolution flag + handler composition + 4 tests
- `crates/assay-mcp/src/server.rs` — conflict_resolution field on OrchestrateRunParams + parse + handler composition + 3 tests; updated existing test struct literal
- `crates/assay-core/tests/orchestrate_integration.rs` — Updated existing config constructions with conflict_resolution_enabled: false

## Forward Intelligence

### What the next slice should know
- `ConflictResolutionOutput` (the AI response struct) is crate-local to `assay-core::orchestrate::conflict_resolver`. S02's `ConflictResolution` audit type will need to capture the data *returned* by `resolve_conflict()` — consider having `resolve_conflict()` return a richer result type that includes stdout, exit code, resolved file list, and the AI's output, not just `ConflictAction`. Currently `ConflictAction::Resolved(sha)` only carries the SHA.
- `resolve_conflict()` currently returns `ConflictAction` (from assay-types). To add audit trail data, either extend `ConflictAction::Resolved` to carry a struct, or change the return type of `resolve_conflict()` to a new `ConflictResolutionResult` type. The former is simpler but changes a public type; the latter keeps the types clean.
- The validation command (S02's `validation_command` on `ConflictResolutionConfig`) should run *after* `git commit` but *before* returning `ConflictAction::Resolved`. If validation fails, the implementation needs to `git reset --hard HEAD~1` (or equivalent) to undo the resolution commit and then `git merge --abort` to restore clean state. This two-step rollback is tricky — test it carefully.
- `conflict_resolution_enabled` on `MergeRunnerConfig` was added specifically to avoid changing the public `merge_completed_sessions()` signature mid-slice. S02 should consider whether this field should become part of `ConflictResolutionConfig` (i.e., `config.enabled` replaces the standalone bool) for cleaner cohesion.

### What's fragile
- The `try_wait()` polling loop for timeout enforcement — if the system is heavily loaded, the 100ms poll interval may allow slightly over-timeout execution. Not critical for correctness (kill is called on timeout) but the elapsed time in error messages may exceed the configured timeout by up to 100ms.
- `git commit --no-edit` relies on `MERGE_HEAD` being present to auto-generate a merge commit message. If the working tree somehow loses `MERGE_HEAD` between the conflict detection and the commit, the commit will succeed but produce a non-merge commit (no second parent). The SHA verification in the merge runner does not validate commit topology — only that the SHA is valid.
- Handler panic safety uses `catch_unwind(AssertUnwindSafe)`. This is correct but `AssertUnwindSafe` suppresses the compiler's unwind-safety check — the handler closure must not contain `Mutex` or `RefCell` in a way that leaves shared state corrupted on panic.

### Authoritative diagnostics
- `MergeReport.results[i].status` + `MergeReport.results[i].error` — most complete picture of what happened per session; `ConflictSkipped` with an error message indicates a resolution attempt that failed
- `tracing::warn` with `session_name` + `error` fields in `resolve_conflict()` — first place to look when a real Claude invocation fails; includes raw_output_len for distinguishing empty vs garbage responses
- `MERGE_HEAD` file in `.git/` — if present after merge runner completes, a merge --abort failed or was skipped (indicates a bug in the cleanup path)

### What assumptions changed
- T02 planned a `ConflictResolutionOutput` schema snapshot in assay-types. The type ended up crate-local in assay-core (D046) so no assay-types snapshot was needed — only the `ConflictResolutionConfig` snapshot was added. S02's audit trail type will be in assay-types and will require a snapshot.
