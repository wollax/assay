---
estimated_steps: 8
estimated_files: 4
---

# T02: Implement resolve_conflict() Audit Capture + Validation + Update All Callers

**Slice:** S02 — Audit Trail, Validation & End-to-End
**Milestone:** M003

## Description

The signature-change task. Changes `resolve_conflict()` from `-> ConflictAction` to `-> ConflictResolutionResult`, implementing two new behaviors: (1) capture of original and resolved file contents into a `ConflictResolution` audit record, and (2) execution of an optional validation command after the merge commit with `git reset --hard HEAD~1` rollback on failure. Updates all callers in the merge runner, CLI, and MCP server. Updates all inline tests that reference the old return type.

The critical correctness invariant: `repo_clean` controls whether the merge runner calls `git merge --abort`. When `true`, the resolver already cleaned up (via reset); when `false`, the merge is still in progress and the runner must abort it. This flag is only relevant on `Skip` or `Abort` actions — on `Resolved`, the merge was committed and cleanup isn't needed.

Key implementation note for validation rollback: after `git commit --no-edit`, `MERGE_HEAD` is consumed. Calling `git merge --abort` at that point would fail (exit 128, "not in merge state"). The rollback must be `git reset --hard HEAD~1` only. Return `repo_clean: true` so the merge runner skips its `git merge --abort` call.

## Steps

1. **Extract `run_validation_command()` helper in `conflict_resolver.rs`**:
   - Signature: `fn run_validation_command(cmd: &str, work_dir: &Path, timeout_secs: u64) -> Result<(), String>`
   - Splits the shell command string using a simple approach: if the command contains a space, use `sh -c "<cmd>"`; otherwise invoke directly. Use `std::process::Command` with `try_wait()` polling at 100ms (same pattern as `wait_with_timeout()`). Non-zero exit returns `Err(format!("validation command exited with code {:?}", code))`. Binary not found returns `Err("validation command not found: ...")`.
   - Add unit tests: `run_validation_command_success` (`echo ok`, expect `Ok(())`), `run_validation_command_failure` (`sh -c 'exit 1'`, expect `Err(_)`), `run_validation_command_not_found` (`nonexistent_binary_xyz`, expect `Err(_)`)

2. **Change `resolve_conflict()` to return `ConflictResolutionResult`**:
   - Before writing resolved files: capture original file contents as `Vec<ConflictFileContent>` by reading each file in `conflicting_files` (same try `read_to_string` pattern as `build_conflict_prompt`)
   - After successfully writing all resolved files: collect `resolved_contents: Vec<ConflictFileContent>` from `resolution.resolved_files` (the AI-provided content that was written to disk)
   - Expose resolver stdout: modify `spawn_resolver()` or restructure to pass stdout through to the calling function. Capture the stdout string and store as `resolver_stdout`
   - After `git commit --no-edit` succeeds and SHA is obtained: if `config.validation_command` is `Some(cmd)`, call `run_validation_command(cmd, work_dir, config.timeout_secs)`:
     - On `Err(e)`: log `tracing::warn!`, call `git_command(&["reset", "--hard", "HEAD~1"], work_dir)` — on reset failure, return a hard error (propagate as `Err(...)` to the caller, not `Skip`); on reset success, return `ConflictResolutionResult { action: ConflictAction::Skip, audit: None, repo_clean: true }`
   - On success (validation passed or not configured): return `ConflictResolutionResult { action: Resolved(sha), audit: Some(ConflictResolution { session_name: session_name.to_string(), conflicting_files: conflicting_files.to_vec(), original_contents, resolved_contents, resolver_stdout, validation_passed: config.validation_command.as_ref().map(|_| true) }), repo_clean: false }`
   - On any error before the commit (subprocess, parse, git add, git commit): return `ConflictResolutionResult { action: Skip, audit: None, repo_clean: false }`

3. **Update the test `resolve_conflict_returns_skip_when_claude_not_found`**:
   - Change `assert_eq!(result, ConflictAction::Skip)` to:
     ```rust
     assert_eq!(result.action, ConflictAction::Skip);
     assert!(!result.repo_clean);
     assert!(result.audit.is_none());
     ```

4. **Change handler type in `merge_completed_sessions()`**:
   - Change `H: Fn(&str, &[String], &ConflictScan, &Path) -> ConflictAction` to `H: Fn(&str, &[String], &ConflictScan, &Path) -> ConflictResolutionResult`

5. **Update the two-phase path in `merge_runner.rs`**:
   - Destructure `ConflictResolutionResult { action, audit, repo_clean }` from the `catch_unwind` result
   - For `Resolved(sha)` case: push audit to `report.resolutions` when `Some(audit)` — `report.resolutions.push(audit)`
   - For `Skip` case: if `!repo_clean`, call `git merge --abort`; if `repo_clean`, skip the abort
   - For `Abort` case: if `!repo_clean`, call `git merge --abort` before stopping the loop; if `repo_clean`, skip the abort
   - For panic case: the panic always means no commit occurred, so always call `git merge --abort` (same as before)
   - Add `resolutions: Vec::new()` initialization at the top of the merge loop; construct `MergeReport` with `resolutions` field

6. **Update `default_conflict_handler()`**:
   - Change return type to `impl Fn(&str, &[String], &ConflictScan, &Path) -> ConflictResolutionResult`
   - Return `ConflictResolutionResult { action: ConflictAction::Skip, audit: None, repo_clean: false }`

7. **Update all inline handler closures in `merge_runner.rs` tests**:
   - `resolver_handler` closure in `test_merge_runner_conflict_resolution_with_live_tree`: change return type; wrap the `ConflictAction::Resolved(sha)` in `ConflictResolutionResult { action: Resolved(sha), audit: None, repo_clean: false }`
   - `skip_handler` and `panic_handler` and `abort_handler` in other tests: wrap similarly
   - The `skip_handler` in `test_merge_runner_conflict_resolution_handler_failure` returns `{ action: Skip, audit: None, repo_clean: false }` (merge runner will then call git merge --abort since repo_clean is false)

8. **Update CLI and MCP handler closures**:
   - `crates/assay-cli/src/commands/run.rs`: the `auto` branch handler closure currently calls `resolve_conflict(...)` and returns `ConflictAction`. Since `resolve_conflict()` now returns `ConflictResolutionResult`, the closure just passes it through (return type inference handles it automatically if the closure signature is inferred)
   - `crates/assay-mcp/src/server.rs`: same — the handler closure wrapping `resolve_conflict()` now returns `ConflictResolutionResult`; remove any explicit `ConflictAction` type annotation on the closure
   - Run `just ready` to confirm all suites pass

## Must-Haves

- [ ] `resolve_conflict()` returns `ConflictResolutionResult` with original and resolved file contents in `audit`
- [ ] `run_validation_command()` extracted as a testable helper with 3 unit tests
- [ ] Validation failure triggers `git reset --hard HEAD~1` and returns `repo_clean: true`
- [ ] `git reset --hard HEAD~1` failure is a hard error (not swallowed as Skip)
- [ ] Merge runner checks `repo_clean` before calling `git merge --abort`
- [ ] `MergeReport.resolutions` populated from `audit` in the `Resolved` path
- [ ] `default_conflict_handler()` updated to return `ConflictResolutionResult`
- [ ] All existing tests pass with updated handler types
- [ ] `just ready` green

## Verification

- `cargo test -p assay-core resolve_conflict` — `run_validation_command_success`, `run_validation_command_failure`, `run_validation_command_not_found`, updated `resolve_conflict_returns_skip_when_claude_not_found` all pass
- `cargo test -p assay-core merge_runner` — all 6 existing merge runner integration tests pass with updated handler closures
- `cargo test -p assay-cli run` — 16 existing tests pass
- `cargo test -p assay-mcp orchestrate_run` — 8 existing tests pass
- `just ready` — fully green

## Observability Impact

- Signals added/changed: `tracing::warn!` with `session_name`, `validation_cmd`, `reason` on validation failure; `tracing::info!` now includes `validation_passed: true` on successful resolution with validation configured; `tracing::warn!` with `error` on `git reset --hard HEAD~1` failure (before propagating hard error)
- How a future agent inspects this: `MergeSessionResult.error` for `ConflictSkipped` now distinguishes "validation failed" (when repo_clean=true) from "handler returned skip" (repo_clean=false, merge aborted); `MergeReport.resolutions` provides full original/resolved content for post-hoc inspection
- Failure state exposed: `ConflictResolution.validation_passed: None` when no validation configured; `audit: None` when resolver skipped or validation failed (the absence of an audit record indicates the resolution was not completed)

## Inputs

- T01 outputs: `ConflictResolution`, `ConflictFileContent`, `ConflictResolutionResult` structs defined
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — existing `resolve_conflict()`, `spawn_resolver()`, `wait_with_timeout()`, `git_command()` usage
- `crates/assay-core/src/orchestrate/merge_runner.rs` — existing two-phase conflict path with `catch_unwind`; existing `default_conflict_handler()`
- S01 summary — `repo_clean` semantics: only relevant when action is Skip/Abort; `git commit --no-edit` consumes `MERGE_HEAD` so `git merge --abort` would fail afterward
- Research doc constraint: `git reset --hard HEAD~1` on validation failure (not `git merge --abort`)

## Expected Output

- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — `resolve_conflict()` returns `ConflictResolutionResult`; `run_validation_command()` extracted with 3 tests; existing test assertion updated
- `crates/assay-core/src/orchestrate/merge_runner.rs` — handler type changed; `repo_clean` check; `resolutions` populated; `default_conflict_handler()` updated; all inline test closures updated
- `crates/assay-cli/src/commands/run.rs` — handler closure passes through `ConflictResolutionResult`
- `crates/assay-mcp/src/server.rs` — handler closure passes through `ConflictResolutionResult`
