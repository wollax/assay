# M003/S01: AI Conflict Resolution — Research

**Date:** 2026-03-17

## Summary

S01 is **substantially complete**. All core deliverables identified in the S01→S02 boundary map have been implemented and tested. The two-phase merge lifecycle, conflict resolver subprocess, CLI flag, MCP parameter, and integration tests with scripted resolvers against real git repos are already in place. `just ready` passes with zero failures.

The work that remains is verification and documentation, not implementation. S01 can move directly to a plan-and-verify pass: confirm each boundary-map deliverable compiles, has test coverage, and the integration test proves the scripted resolver path end-to-end.

**What's already built:**
1. `merge_execute()` in `merge.rs` accepts `abort_on_conflict: bool` — when `false`, leaves the repo in conflict state (MERGE_HEAD present, files contain markers). Tested by `test_merge_execute_two_phase_*` integration tests.
2. `merge_completed_sessions()` in `merge_runner.rs` has `MergeRunnerConfig.conflict_resolution_enabled` controlling two-phase vs. legacy path. Two-phase path wraps handler in `catch_unwind` for panic safety, aborts on any handler failure or invalid SHA. Tested by `test_merge_runner_conflict_resolution_with_live_tree` and `test_merge_runner_conflict_resolution_handler_failure`.
3. `conflict_resolver.rs` in `assay-core/src/orchestrate/` implements `resolve_conflict()` — a fully sync `std::process::Command`-based subprocess (D043 compliant) that: reads conflicted file contents, constructs a structured `build_conflict_prompt()`, spawns `claude -p --json-schema`, parses `ConflictResolutionOutput`, writes resolved files, stages, and commits.
4. `ConflictResolutionConfig` type in `assay-types/src/orchestrate.rs` with schema registration and snapshot test.
5. CLI `--conflict-resolution auto|skip` flag on `assay run`, wired into `execute_orchestrated()` — `auto` constructs a `ConflictResolutionConfig{enabled:true}` and passes `resolve_conflict` closure to `merge_completed_sessions`.
6. MCP `orchestrate_run` tool accepts optional `conflict_resolution: Option<String>` parameter (`"auto"` or `"skip"`), routed to the same two-phase path.

**What's missing / to verify:**
- No dedicated end-to-end integration test in `orchestrate_integration.rs` exercises `resolve_conflict()` directly (all existing integration tests use mock/scripted handlers, not the AI path). The scripted-resolver integration test in `merge_runner.rs` satisfies the S01 boundary requirement ("proven by integration test with a scripted resolver subprocess against a real git repo").
- No distinct error variants for resolution-specific failures — failures are surfaced as `ConflictAction::Skip` with tracing warnings. This is by design (graceful degradation), but may need explicit error types if S02 needs to distinguish resolution failure from deliberate skip in the audit trail.
- `MarkerType` Display impl referenced in `build_conflict_prompt()` (via `marker.marker_type` format) — confirm the Display impl exists or the format string is correct.

## Recommendation

**No new implementation needed for S01.** The slice is effectively done. The plan task should:
1. Write a summary that enumerates the deliverables, confirms test coverage, and notes that `just ready` passes.
2. Identify the two gaps that need attention before S02: (a) confirm the `MarkerType` Display format is correct, (b) decide whether S01 needs explicit error variants or whether the graceful-Skip approach is sufficient for S02's audit trail needs.
3. Update the requirements tracking for R026 (mark S01 deliverables as validated).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Two-phase conflict state | `merge_execute()` `abort_on_conflict: bool` | Already implemented and tested in `merge.rs` |
| Conflict marker scanning | `scan_files_for_markers()` in `merge.rs` | Already used by the merge runner to build `ConflictScan` |
| Sync subprocess timeout | `wait_with_timeout()` in `conflict_resolver.rs` | Already implemented with `try_wait()` polling loop |
| Prompt construction | `build_conflict_prompt()` in `conflict_resolver.rs` | Already reads live file contents from working tree |
| JSON schema for structured output | `conflict_resolution_schema_json()` (LazyLock) | Same pattern as `evaluator_schema_json()` in `evaluator.rs` |
| Panic-safe handler dispatch | `catch_unwind(AssertUnwindSafe(...))` in `merge_runner.rs` | Already wraps the handler call in two-phase path |

## Existing Code and Patterns

- `crates/assay-core/src/merge.rs` — `merge_execute(project_root, branch, message, abort_on_conflict: bool)`. When `abort_on_conflict=false`, returns `was_conflict=true` with MERGE_HEAD intact. Integration test: `test_merge_execute_two_phase_conflict_leaves_tree_conflicted`.
- `crates/assay-core/src/orchestrate/merge_runner.rs` — `MergeRunnerConfig.conflict_resolution_enabled`. Two-phase path (lines ~150-220): `catch_unwind`, `git rev-parse --verify <sha>` confirmation, graceful `git merge --abort` on handler failure/panic/invalid SHA.
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — Full implementation. Key types: `ConflictResolutionOutput`, `ResolvedFile`. Key functions: `resolve_conflict()`, `build_conflict_prompt()`, `build_conflict_system_prompt()`, `spawn_resolver()`, `parse_resolver_output()`. Has 15 unit tests covering schema, deserialization, prompt construction, parse failure paths, and the graceful-Skip-on-missing-claude path.
- `crates/assay-types/src/orchestrate.rs` — `ConflictResolutionConfig` with `enabled`, `model` (default `"claude-sonnet-4-20250514"`), `timeout_secs` (default 120). Schema registered and snapshot-tested.
- `crates/assay-cli/src/commands/run.rs` — `ConflictResolutionMode::Auto|Skip`, `--conflict-resolution` clap flag, `execute_orchestrated()` routing. 6 CLI tests cover flag parsing.
- `crates/assay-mcp/src/server.rs` — `OrchestrateRunParams.conflict_resolution: Option<String>`, validated at line ~2829. 5 MCP tests cover `conflict_resolution` parameter paths.

## Constraints

- **Sync-only subprocess (D043)** — `resolve_conflict()` uses `std::process::Command` + polling `wait_with_timeout()`. Do not add async. The 100ms poll interval means timeout granularity is ±100ms.
- **Graceful-Skip on all failures** — `resolve_conflict()` returns `ConflictAction::Skip` on any subprocess or parse error. The caller (`merge_completed_sessions`) then calls `git merge --abort`. This means conflict resolution failures are silent in the `MergeReport` — they appear as `ConflictSkipped` identical to an intentional skip.
- **`claude -p --json-schema` contract** — The subprocess flags match `evaluator.rs`: `-p`, `--output-format json`, `--json-schema`, `--system-prompt`, `--tools ""`, `--max-turns 1`, `--model`, `--no-session-persistence`. Any changes to the Claude CLI interface will break both.
- **MERGE_HEAD lifecycle** — The two-phase path requires MERGE_HEAD to be present when the handler runs. If the handler commits successfully, MERGE_HEAD is cleared by git. If the handler calls `git merge --abort`, MERGE_HEAD is cleared. Any other exit path leaves MERGE_HEAD present — `merge_completed_sessions` must `git merge --abort` on any non-Resolved return.
- **`ConflictResolutionOutput.resolved_files` must cover all conflicting files** — If the AI returns a subset, the unstaged conflict markers remain and `git commit --no-edit` will fail. `resolve_conflict()` currently does not validate completeness before committing.

## Common Pitfalls

- **AI returns partial file list** — If `resolved_files` covers only some conflicting files, `git commit --no-edit` will fail because unresolved markers remain. Current code fails at the `git_command(&["commit", "--no-edit"], ...)` call and returns `Skip`. S02 audit trail should record this failure mode explicitly.
- **`spawn_resolver` stdout/stderr reading race** — The current `wait_with_timeout()` implementation reads stdout/stderr after `try_wait()` returns `Some(status)`. For very large outputs, the process may have exited but the pipe buffer not yet drained. This is unlikely in practice (structured JSON output is small) but worth noting.
- **Schema snapshot drift** — `ConflictResolutionOutput` and `ResolvedFile` are not registered in the schema registry (they're internal to `conflict_resolver.rs`, not in `assay-types`). If S02 adds these to the public API, they need schema registration and snapshot tests.
- **`git commit --no-edit` requires a commit message** — When MERGE_HEAD is present, git uses the auto-generated merge commit message. This works correctly; no `--message` flag is needed.

## Open Risks

- **AI resolution quality is not validated in S01** — `resolve_conflict()` writes whatever the AI returns without semantic validation. S02's post-resolution validation command (`cargo check`) addresses this. Risk level: medium — the merge commit will be syntactically complete (markers removed) but may have semantic errors.
- **Claude CLI output format may change** — The `structured_output` envelope key and `is_error` flag are parsed by `parse_resolver_output()`. If Claude CLI changes its output format, both the evaluator and the conflict resolver break simultaneously. Monitoring for `claude` CLI updates is prudent.
- **Integration test does not invoke real Claude** — `test_merge_runner_conflict_resolution_with_live_tree` uses a scripted Rust closure as the handler, not `resolve_conflict()`. The real `claude` binary is not called in CI. This is the correct design (deterministic tests), but means the full AI path is only tested manually via UAT.
- **Timeout implementation is polling-based** — `wait_with_timeout()` polls every 100ms. For a 120s timeout, this is 1,200 `try_wait()` calls. Acceptable, but a blocking `wait()` on a thread with a channel-based timeout would be more efficient.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | `modu-ai/moai-adk@moai-lang-rust` | available (56 installs) — not needed, codebase patterns are sufficient |
| Git merge internals | none | n/a — `std::process::Command` + git CLI patterns are already established |

No skills are required. All patterns (sync subprocess, JSON schema, prompt construction, two-phase git lifecycle) are established in the codebase.

## Sources

- Two-phase merge implementation (source: `crates/assay-core/src/merge.rs`, `merge_execute()` + tests)
- Conflict resolver implementation (source: `crates/assay-core/src/orchestrate/conflict_resolver.rs`)
- Merge runner two-phase path (source: `crates/assay-core/src/orchestrate/merge_runner.rs`, lines ~135-230)
- CLI flag wiring (source: `crates/assay-cli/src/commands/run.rs`, `execute_orchestrated()`)
- MCP parameter wiring (source: `crates/assay-mcp/src/server.rs`, lines ~2829-2970)
- `ConflictResolutionConfig` type and schema (source: `crates/assay-types/src/orchestrate.rs`)
- D043 (sync subprocess), D044 (two-phase lifecycle) (source: `.kata/DECISIONS.md`)
- `just ready` passing (source: local verification, 2026-03-17)
