# S01: AI Conflict Resolution

**Goal:** `merge_completed_sessions()` with an AI conflict handler resolves real git merge conflicts — proven by integration test with a scripted resolver subprocess against a real git repo. CLI `--conflict-resolution auto` flag routes to the handler. Existing merge behavior (auto-abort + skip) is unchanged by default.

**Demo:** An integration test creates a real git repo with two branches that conflict, invokes `merge_completed_sessions()` with a conflict handler that receives a live conflicted working tree, resolves the conflict by stripping markers/staging/committing, and the merge report shows `Merged` status with a valid commit SHA. A second test proves the CLI `--conflict-resolution auto` flag parses and composes the handler. Existing tests pass unchanged.

## Must-Haves

- `merge_execute()` supports two-phase lifecycle: new `abort_on_conflict: bool` parameter (default `true`) preserves existing behavior; when `false`, returns `MergeExecuteResult` with conflict details while working tree is still conflicted
- `merge_completed_sessions()` manages conflict lifecycle: on conflict with resolution enabled, calls handler with live conflicted tree; on handler failure/panic, runs `git merge --abort`; on handler success, verifies commit SHA
- Sync `resolve_conflict()` function: reads conflicted files, builds structured prompt, spawns `claude -p` subprocess via `std::process::Command`, parses JSON response, writes resolved files, stages + commits, returns `ConflictAction::Resolved(sha)`
- `ConflictResolutionConfig` type: enabled/disabled, model, timeout — controls resolution behavior (in assay-types with schema snapshot)
- CLI `--conflict-resolution auto|skip` flag on `assay run` composing the appropriate handler closure
- MCP `orchestrate_run` tool accepts `conflict_resolution` parameter
- Integration test with real git repo and scripted resolver subprocess proves end-to-end conflict resolution
- All existing merge runner and merge.rs tests pass unchanged (default `abort_on_conflict: true`)

## Proof Level

- This slice proves: integration (real git repos with actual merge conflicts resolved by a subprocess)
- Real runtime required: yes (git operations on real repos in temp dirs)
- Human/UAT required: yes — real Claude invocation resolving a genuine conflict is UAT (not automated in this slice; scripted resolver used for deterministic testing)

## Verification

- `cargo test -p assay-core merge_execute_two_phase` — tests for the new `abort_on_conflict: false` path
- `cargo test -p assay-core merge_runner_conflict_resolution` — integration test: real git repo → conflicting branches → handler with live tree → resolved merge
- `cargo test -p assay-core resolve_conflict` — unit tests for prompt construction, response parsing, subprocess error handling
- `cargo test -p assay-types schema_snapshots` — schema snapshots for `ConflictResolutionConfig` and any modified types
- `cargo test -p assay-cli run` — CLI flag parsing tests for `--conflict-resolution`
- `cargo test -p assay-mcp orchestrate_run` — MCP parameter deserialization for `conflict_resolution`
- `just ready` — all 1183+ existing tests continue to pass

## Observability / Diagnostics

- Runtime signals: `resolve_conflict()` returns structured `ConflictResolutionResult` with resolver stdout/stderr, exit code, duration, and resolved file list — all available for logging by the caller
- Inspection surfaces: `MergeSessionResult` already carries `merge_sha` and `error` fields; conflict resolution success/failure is visible in `MergeReport` per-session results
- Failure visibility: Handler failure (subprocess crash, timeout, parse error) produces a descriptive error string in `MergeSessionResult.error`; the merge runner falls back to `git merge --abort` and records the session as `ConflictSkipped`
- Redaction constraints: Resolver subprocess stdout may contain AI-generated content — no secrets, but S02 will formalize audit trail storage

## Integration Closure

- Upstream surfaces consumed: `merge_execute()` in `merge.rs`, `merge_completed_sessions()` in `merge_runner.rs`, `ConflictAction` enum in `assay-types/orchestrate.rs`, evaluator subprocess patterns from `evaluator.rs`, CLI `RunCommand` in `run.rs`, MCP `OrchestrateRunParams` in `server.rs`
- New wiring introduced in this slice: two-phase `merge_execute()` parameter, `resolve_conflict()` sync function, `ConflictResolutionConfig` type, CLI flag → handler closure composition, MCP param → handler closure composition
- What remains before the milestone is truly usable end-to-end: S02 adds audit trail (`ConflictResolution` on `MergeReport`), post-resolution validation command, `orchestrate_status` resolution details, and end-to-end CLI integration test through the full pipeline

## Tasks

- [x] **T01: Two-phase merge_execute and conflict resolution types** `est:1h`
  - Why: The foundation — `merge_execute()` currently auto-aborts on conflict, preventing any handler from resolving. This adds the `abort_on_conflict` parameter and the `ConflictResolutionConfig` type needed by all downstream tasks.
  - Files: `crates/assay-core/src/merge.rs`, `crates/assay-types/src/orchestrate.rs`, `crates/assay-types/tests/schema_snapshots.rs`
  - Do: Add `abort_on_conflict: bool` parameter to `merge_execute()` (default `true`). When `false`, skip the `git merge --abort` block and return `MergeExecuteResult` with conflict details while tree is still conflicted. Update all existing callers to pass `true`. Add `ConflictResolutionConfig` type to `assay-types/orchestrate.rs` with `enabled`, `model`, `timeout_secs` fields + schema snapshot. Add integration tests with real git repos proving both paths.
  - Verify: `cargo test -p assay-core merge` passes (existing + new two-phase tests); `cargo test -p assay-types schema_snapshots` passes with new snapshot
  - Done when: `merge_execute(..., false)` returns a `MergeExecuteResult` with `was_conflict: true` while the working tree still has conflict markers; `merge_execute(..., true)` behaves identically to the old version; `ConflictResolutionConfig` compiles with serde/schemars derives and has a locked schema snapshot

- [x] **T02: Sync conflict resolver function** `est:1h30m`
  - Why: The AI resolver itself — reads conflicted files from a live working tree, builds a structured prompt, spawns `claude -p` synchronously, parses the JSON response, writes resolved files, stages, and commits. This is the core new capability.
  - Files: `crates/assay-core/src/orchestrate/conflict_resolver.rs` (new), `crates/assay-core/src/orchestrate/mod.rs`, `crates/assay-types/src/orchestrate.rs`
  - Do: Create `conflict_resolver.rs` module. Define `ConflictResolutionOutput` serde type for the AI response schema (resolved file contents). Implement `resolve_conflict(session_name, conflicting_files, conflict_scan, work_dir, config) -> ConflictAction` using `std::process::Command` to spawn `claude -p --json-schema`. Build prompt from conflict scan + full file contents with markers. Parse response, write files, `git add`, `git commit`. Return `ConflictAction::Resolved(sha)`. On any error (subprocess crash, parse failure, claude not found), return `ConflictAction::Skip` with logged error. Add unit tests for prompt construction and response parsing (mock the subprocess by testing the functions that build/parse, not the spawn itself).
  - Verify: `cargo test -p assay-core conflict_resolver` passes; `cargo test -p assay-types schema_snapshots` passes
  - Done when: `resolve_conflict()` compiles and has tested prompt building and response parsing paths; `ConflictResolutionOutput` has a locked schema snapshot; the function handles subprocess not-found gracefully

- [x] **T03: Wire conflict handler into merge runner lifecycle** `est:1h`
  - Why: Connects T01 (two-phase merge) and T02 (resolver) into the merge runner loop. The merge runner must call `merge_execute(..., abort_on_conflict: false)` when resolution is enabled, invoke the handler with the live conflicted tree, verify the result, and fall back to `git merge --abort` on failure.
  - Files: `crates/assay-core/src/orchestrate/merge_runner.rs`
  - Do: Add `conflict_resolution_enabled: bool` to `MergeRunnerConfig`. When enabled, call `merge_execute(..., false)` on merge. If conflict, invoke handler with live tree. If handler returns `Resolved(sha)`, verify commit with `git rev-parse`. If handler fails or panics (`std::panic::catch_unwind`), run `git merge --abort`. When disabled (default), existing behavior unchanged: `merge_execute(..., true)` + handler receives post-abort scan. Add integration test: real git repo with conflicting branches, scripted resolver handler that strips markers and commits, proves `MergeReport` shows `Merged` status.
  - Verify: `cargo test -p assay-core merge_runner` passes (all existing + new conflict resolution integration test)
  - Done when: Integration test proves: two branches conflict → handler called with live tree → handler resolves + commits → merge report shows `Merged` with valid SHA → repo history has proper merge commit with both parents

- [x] **T04: CLI flag and MCP parameter for conflict resolution** `est:45m`
  - Why: Completes the slice by wiring the conflict resolver into both user-facing entry points. Without this, the resolver exists but isn't accessible.
  - Files: `crates/assay-cli/src/commands/run.rs`, `crates/assay-mcp/src/server.rs`
  - Do: Add `--conflict-resolution auto|skip` flag to `RunCommand` (default `skip`). When `auto`, compose handler closure using `resolve_conflict()` from T02 and pass to `merge_completed_sessions()`. Update `execute_orchestrated()` to pass `conflict_resolution_enabled` to `MergeRunnerConfig`. Add `conflict_resolution` optional parameter to `OrchestrateRunParams` in MCP server. Route through same composition. Add CLI arg parsing tests. Add MCP deserialization test.
  - Verify: `cargo test -p assay-cli run` passes; `cargo test -p assay-mcp orchestrate_run` passes; `just ready` green
  - Done when: `assay run manifest.toml --conflict-resolution auto` compiles and routes to AI handler; MCP `orchestrate_run` with `conflict_resolution: "auto"` deserializes and routes correctly; all existing tests pass unchanged

## Files Likely Touched

- `crates/assay-core/src/merge.rs` — two-phase `merge_execute()`
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — new module: `resolve_conflict()`, prompt builder, response parser
- `crates/assay-core/src/orchestrate/merge_runner.rs` — lifecycle management, `conflict_resolution_enabled` config
- `crates/assay-core/src/orchestrate/mod.rs` — register `conflict_resolver` module
- `crates/assay-types/src/orchestrate.rs` — `ConflictResolutionConfig` type
- `crates/assay-cli/src/commands/run.rs` — `--conflict-resolution` flag
- `crates/assay-mcp/src/server.rs` — `conflict_resolution` parameter on `orchestrate_run`
- `crates/assay-types/tests/schema_snapshots.rs` — new snapshots
