---
id: T02
parent: S01
milestone: M003
provides:
  - Sync resolve_conflict() function that spawns claude -p subprocess for AI conflict resolution
  - ConflictResolutionOutput response type with schemars derive and cached JSON schema
  - Prompt construction that reads full file contents from live working tree
  - Graceful fallback to ConflictAction::Skip on any subprocess error
key_files:
  - crates/assay-core/src/orchestrate/conflict_resolver.rs
  - crates/assay-core/src/orchestrate/mod.rs
key_decisions:
  - ConflictResolutionOutput type kept crate-local to assay-core (not assay-types) since it's an internal subprocess contract, not a persistence/API type
  - Sync subprocess uses try_wait polling with 100ms interval for timeout support (std::process::Child has no native timeout)
  - Commit uses --no-edit flag to auto-use git's merge commit message template when MERGE_HEAD is present
patterns_established:
  - Sync subprocess pattern via std::process::Command (parallel to async evaluator pattern) with write-stdin → poll-wait → collect-output lifecycle
  - Claude envelope parsing reused from evaluator.rs (is_error check → structured_output extraction)
observability_surfaces:
  - tracing::warn on subprocess failure with session_name, error message, and raw_output_len
  - tracing::info on successful resolution with session_name, SHA, and resolved_files count
  - Descriptive error strings for each failure mode: "claude CLI not found", parse errors with detail, timeout with elapsed time
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Sync conflict resolver function

**Built `resolve_conflict()` sync function with prompt builder, subprocess spawn, JSON response parser, file writer, and git stage/commit — returns `ConflictAction::Resolved(sha)` on success or `ConflictAction::Skip` on any error.**

## What Happened

Created `crates/assay-core/src/orchestrate/conflict_resolver.rs` with:

1. **`ConflictResolutionOutput`** response type — `resolved_files: Vec<ResolvedFile>` with schemars derive and `LazyLock<String>` cached JSON schema (same pattern as `EVALUATOR_SCHEMA` in evaluator.rs).

2. **`build_conflict_prompt()`** — reads full file contents from the live working tree (files contain conflict markers), includes session name, file count, conflict marker summary from `ConflictScan`, and instruction text.

3. **`build_conflict_system_prompt()`** — instructs the AI to resolve conflicts by combining both sides, preserve non-conflicting code, remove all markers, and not introduce new code.

4. **`resolve_conflict()`** — the main function. Spawns `claude -p --output-format json --json-schema <schema> --system-prompt <sys> --tools "" --max-turns 1 --model <model> --no-session-persistence` synchronously via `std::process::Command`. Prompt piped via stdin. Parses Claude envelope (`is_error` check, `structured_output` extraction). Writes resolved files, stages with `git add`, commits with `git commit --no-edit` (MERGE_HEAD creates proper merge commit). Returns `ConflictAction::Resolved(sha)`. Any error at any stage returns `ConflictAction::Skip` with tracing::warn.

5. **`spawn_resolver()`** / **`wait_with_timeout()`** — sync subprocess helpers. Uses `try_wait()` polling with 100ms sleep for timeout enforcement. Kills child on timeout.

6. **18 unit tests** covering schema generation, response deserialization (valid, empty, malformed), envelope parsing (valid, is_error, missing structured_output, invalid JSON), prompt construction (session name, file count, disk file contents with markers, unreadable file, marker summary), system prompt, and `resolve_conflict()` returning Skip when claude is not found.

## Verification

- `cargo test -p assay-core --features orchestrate conflict_resolver` — **18 tests passed**
- `cargo test -p assay-core --features orchestrate merge_execute_two_phase` — **2 tests passed** (T01 tests intact)
- `cargo build -p assay-core` — compiles cleanly

### Slice-level verification status (T02 is 2nd of 4 tasks):
- ✅ `cargo test -p assay-core merge_execute_two_phase` — 2 passed
- ⏳ `cargo test -p assay-core merge_runner_conflict_resolution` — not yet (T03)
- ✅ `cargo test -p assay-core resolve_conflict` — 1 passed (resolve_conflict_returns_skip test)
- ✅ `cargo test -p assay-types schema_snapshots` — passes (no new snapshots; type is crate-local)
- ⏳ `cargo test -p assay-cli run` — not yet (T04)
- ⏳ `cargo test -p assay-mcp orchestrate_run` — not yet (T04)

## Diagnostics

- `resolve_conflict()` logs via tracing at warn level on failure (session_name, error, raw_output_len) and info level on success (session_name, sha, resolved_files count)
- Each failure mode produces a distinct error string: "claude CLI not found in PATH", "claude exited with code X: <stderr_snippet>", "resolver timed out after Xs (elapsed: Y.Zs)", "invalid JSON: ...", "missing structured_output...", "structured_output parse error: ..."
- The caller (merge runner in T03) receives `ConflictAction::Skip` on any failure — no panics propagated

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — new module with resolve_conflict(), prompt builders, response type, schema cache, 18 unit tests
- `crates/assay-core/src/orchestrate/mod.rs` — added `pub mod conflict_resolver;`
