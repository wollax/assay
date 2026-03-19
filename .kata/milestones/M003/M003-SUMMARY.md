---
id: M003
provides:
  - Two-phase merge_execute() with abort_on_conflict parameter — working tree stays conflicted for handler resolution
  - ConflictResolutionConfig type in assay-types (enabled, model, timeout_secs, validation_command) with schema snapshot
  - resolve_conflict() sync function in assay-core — spawns claude -p subprocess, parses JSON envelope, writes resolved files, stages and commits, returns ConflictResolutionResult with full audit
  - run_validation_command() helper — sh -c for shell commands, timeout via try_wait polling, rollback via git reset --hard HEAD~1 on failure
  - ConflictResolutionResult pub type in assay-core — action, audit (Option<ConflictResolution>), repo_clean flag
  - ConflictResolution audit record in assay-types — session_name, original_contents (with markers), resolved_contents (clean), resolver_stdout, validation_passed
  - ConflictFileContent helper type for per-file content capture
  - MergeReport.resolutions: Vec<ConflictResolution> — backward-compatible field, persisted to merge_report.json
  - conflict_resolution_enabled on MergeRunnerConfig with panic-safe handler invocation and two-phase lifecycle management
  - CLI --conflict-resolution auto|skip flag on assay run
  - MCP orchestrate_run conflict_resolution: Option<String> parameter
  - MergeReport atomic persistence to .assay/orchestrator/<run_id>/merge_report.json
  - orchestrate_status returns OrchestrateStatusResponse { status, merge_report } wrapper
  - Four locked schema snapshots (ConflictResolutionConfig, ConflictFileContent, ConflictResolution, updated MergeReport)
key_decisions:
  - D043 — Sync subprocess for conflict resolution (std::process::Command, not async run_evaluator)
  - D044 — Two-phase merge lifecycle (abort_on_conflict parameter; default true preserves existing behavior)
  - D045 — Scripted resolver for deterministic testing (real Claude is UAT only)
  - D046 — ConflictResolutionOutput crate-local to assay-core (internal subprocess protocol)
  - D047 — ConflictResolutionMode CLI enum crate-local to assay-cli (presentation concern)
  - D048 — MCP conflict_resolution as Option<String> (matches failure_policy/merge_strategy pattern)
  - D049 — ConflictResolutionResult as pub type in assay-core (function return type, not persistence contract)
  - D050 — Handler type in merge_completed_sessions changes to ConflictResolutionResult
  - D051 — OrchestrateStatusResponse is a local struct in server.rs (wrapping without changing locked OrchestratorStatus)
patterns_established:
  - Two-phase merge lifecycle: abort_on_conflict=false leaves working tree conflicted; handler resolves in-place; runner verifies SHA, falls back to merge --abort on failure
  - Sync subprocess with timeout: try_wait() polling at 100ms intervals; kill on timeout; collect stdout/stderr after exit
  - Validation rollback: git reset --hard HEAD~1 after bad resolution; Abort (not Skip) if reset fails — hard error propagates
  - Panic-safe handler invocation: catch_unwind(AssertUnwindSafe) around conflict handler; panic → descriptive error + merge --abort
  - Conditional handler composition at CLI/MCP call site: match on mode, compose appropriate closure; no Box<dyn Fn> overhead
  - New optional audit-trail fields on deny_unknown_fields structs use serde(default) + skip_serializing_if
  - Atomic MergeReport persistence: NamedTempFile + rename alongside state.json in .assay/orchestrator/<run_id>/
  - inspect_err for side effects (kill/wait) on Result before propagating with ? — required by clippy::manual_inspect
observability_surfaces:
  - .assay/orchestrator/<run_id>/merge_report.json — full ConflictResolution records with original_contents (markers) and resolved_contents (clean)
  - orchestrate_status MCP tool — always returns { "status": OrchestratorStatus, "merge_report": null | MergeReport }
  - tracing::info! with session_name, sha, validation_cmd, validation_passed=true on successful resolution
  - tracing::warn! with session_name, validation_cmd, reason on validation failure before rollback
  - tracing::warn! in orchestrate_run when merge_report.json write fails (non-fatal); in orchestrate_status when parse fails
  - MergeReport.results[i].status + error — ConflictSkipped with error detail vs Merged with merge_sha
  - MERGE_HEAD file in .git/ — if present after merge runner completes, a cleanup bug exists
requirement_outcomes:
  - id: R026
    from_status: active
    to_status: validated
    proof: S01 integration tests prove conflicting branches → live conflicted tree (abort_on_conflict=false) → scripted handler strips markers, stages, commits → MergeReport shows Merged with valid 2-parent merge SHA. S02 adds audit trail captured in MergeReport.resolutions. CLI --conflict-resolution auto flag + MCP conflict_resolution parameter both route to the handler. just ready passes with 0 warnings.
  - id: R028
    from_status: active
    to_status: validated
    proof: S02 run_validation_command() unit tests (success/failure/not_found), integration test with validation_command:"sh -c 'exit 1'" produces Skip + empty resolutions, git reset --hard HEAD~1 rollback proven. just ready passes.
  - id: R029
    from_status: active
    to_status: validated
    proof: S02 test_merge_resolutions_audit_trail integration test asserts MergeReport.resolutions[0] has session_name, original_contents (with conflict markers), resolved_contents (clean), resolver_stdout. merge_report.json persisted to .assay/orchestrator/<run_id>/. orchestrate_status returns merge_report wrapper. just ready passes.
duration: ~180min (S01: ~70min × 4 tasks; S02: ~110min × 3 tasks)
verification_result: passed
completed_at: 2026-03-17
---

# M003: Conflict Resolution & Polish

**Two-phase merge lifecycle, sync AI conflict resolution with audit trail and post-resolution validation, and full MergeReport observability — making the multi-agent merge pipeline fully autonomous with a complete diagnostic record.**

## What Happened

**S01** built the infrastructure foundation. `merge_execute()` gained an `abort_on_conflict` parameter: when `false`, the auto-abort block is skipped and the working tree remains conflicted, enabling a downstream handler to resolve in-place. `ConflictResolutionConfig` was added to `assay-types` with its schema snapshot locked. The core resolver, `resolve_conflict()`, was built in a new `conflict_resolver.rs` module: it spawns `claude -p --output-format json --json-schema <schema>` synchronously via `std::process::Command`, pipes a structured prompt via stdin (file contents with markers, session name, conflict scan summary), parses the Claude envelope, writes resolved files, stages with `git add`, and commits with `git commit --no-edit` (using `MERGE_HEAD` to produce a proper 2-parent merge commit). The merge runner gained `conflict_resolution_enabled` on `MergeRunnerConfig` and a panic-safe two-phase lifecycle: on conflict, the handler is invoked via `catch_unwind(AssertUnwindSafe)`; SHA verified on success; `git merge --abort` on any failure path. CLI got `--conflict-resolution auto|skip` and MCP got `conflict_resolution: Option<String>`, both composing the handler at the call site.

**S02** completed the feature. `ConflictResolution` (audit record) and `ConflictFileContent` (per-file content) were added to `assay-types`. `MergeReport` gained `resolutions: Vec<ConflictResolution>` as a backward-compatible optional field. `ConflictResolutionConfig` gained `validation_command: Option<String>`. `resolve_conflict()` was extended to return `ConflictResolutionResult { action, audit, repo_clean }` instead of just `ConflictAction` — capturing original file contents before resolution, resolved contents after, and resolver stdout. `run_validation_command()` was extracted as a helper: runs `sh -c` for shell commands, direct invocation otherwise, with timeout polling and `git reset --hard HEAD~1` rollback on non-zero exit. The merge runner handler type was updated to `-> ConflictResolutionResult`, with audit records accumulated into `resolutions` on the `Resolved` path. `MergeReport` is now atomically persisted to `.assay/orchestrator/<run_id>/merge_report.json` after each orchestration run. `orchestrate_status` returns `OrchestrateStatusResponse { status, merge_report }` — `merge_report` is always present in the response (null when absent), so callers never need key-existence checks.

The two slices connected cleanly: S02 consumed S01's two-phase lifecycle and `resolve_conflict()`, extending the return type without breaking existing callers. CLI and MCP handler closures required no explicit return type annotation — the type was inferred from `resolve_conflict()`.

## Cross-Slice Verification

**Success criterion 1 — Multi-session orchestration with conflict → auto-resolved → clean merge commit:**
Proven by `test_merge_resolutions_audit_trail` integration test (S02): creates two branches modifying the same file with overlapping changes, calls `merge_completed_sessions()` with a scripted handler that strips conflict markers, asserts `report.resolutions.len() == 1` and `report.results[0].status == Merged` with a valid merge SHA. Real Claude invocation is manual UAT only (scripted resolver proves lifecycle mechanics).

**Success criterion 2 — Conflict handler receives live conflicted working tree:**
Proven by the integration test suite using `abort_on_conflict: false`. `MERGE_HEAD` is present when the handler is called, and `git commit --no-edit` produces a 2-parent merge commit. The merge runner tests also verify that `git rev-parse --verify HEAD` returns the expected SHA after resolution.

**Success criterion 3 — Resolution details recorded in MergeReport:**
Proven by `test_merge_resolutions_audit_trail`: `report.resolutions[0].original_contents[j].content` contains conflict markers; `resolved_contents[j].content` is clean. `resolver_stdout` is populated. `merge_report.json` is persisted to disk and read back by `orchestrate_status_reads_merge_report_when_present` MCP test.

**Success criterion 4 — Optional validation command rejects bad resolutions:**
Proven by `run_validation_command_failure` unit test (non-zero exit → rollback) and integration test with `validation_command: "sh -c 'exit 1'"` producing `Skip` + empty `resolutions`. The `git reset --hard HEAD~1` rollback path is exercised. `Abort` is returned (not `Skip`) if the rollback itself fails.

**Success criterion 5 — CLI flag and MCP parameter control AI resolution:**
Proven by 4 CLI tests (`--conflict-resolution auto` parses, routes to handler; `--conflict-resolution skip` is default; invalid values produce errors) and 3 MCP tests (`conflict_resolution: "auto"` deserializes and routes; `conflict_resolution: "skip"` is default; invalid values produce errors).

**Success criterion 6 — Existing merge runner tests pass (default behavior unchanged):**
Proven by `just ready` with all 1222 tests passing and 0 warnings. `conflict_resolution_enabled: false` is the default — all prior behavior is preserved.

## Requirement Changes

- R026: active → validated — Two-phase merge lifecycle + sync resolver subprocess + handler wiring + CLI/MCP entry points + integration tests with real git repos. `just ready` green.
- R028: active → validated — `run_validation_command()` with rollback proven by unit and integration tests. Non-zero validation exit → `git reset --hard HEAD~1` → `Skip`. `just ready` green.
- R029: active → validated — `MergeReport.resolutions` populated with full audit data; persisted to `merge_report.json`; surfaced via `orchestrate_status`. Integration test asserts original markers + clean resolved content. `just ready` green.

## Forward Intelligence

### What the next milestone should know
- Real Claude invocation via `assay run --conflict-resolution auto` on a project with genuine overlapping session branches is the only remaining UAT. The sync subprocess path is exercised for the not-found case (graceful `Skip`) but real Claude prompt quality and resolution correctness are unverified by automated tests.
- `orchestrate_status` response shape changed in S02: callers must use `response["status"]["run_id"]`, not `response["run_id"]`. Any external tooling consuming `orchestrate_status` before S02 must be updated.
- `ConflictResolution.validation_passed` is `None` when no validation is configured, `Some(true)` when validation passed. `Some(false)` is never written currently — audit record is only written on success. A follow-up improvement would write the audit with `validation_passed: Some(false)` before rollback for richer diagnostics on rejected resolutions.
- `OrchestrateStatusResponse` (the `orchestrate_status` wrapper) has no schema snapshot — it's a local type in `server.rs`. If its shape changes, only consumer tests will catch the regression.

### What's fragile
- `git commit --no-edit` relies on `MERGE_HEAD` being present to generate a merge commit message. If `MERGE_HEAD` is lost between conflict detection and the commit (e.g., from external git operations), the commit succeeds but produces a non-merge commit. The SHA verifier checks validity but not topology.
- `persist_merge_report()` failure is non-fatal (warn-only). A failed fsync/rename leaves `orchestrate_status` returning `merge_report: null` for a run that resolved conflicts — no recovery mechanism exists.
- `catch_unwind(AssertUnwindSafe)` in the handler panic guard suppresses the compiler's unwind-safety check. Handler closures must not contain `Mutex` or `RefCell` in a way that leaves shared state corrupted on panic.
- The `try_wait()` polling loop at 100ms intervals may report elapsed time exceeding the configured timeout by up to 100ms on heavily loaded systems. Not a correctness issue — kill is called correctly on timeout.

### Authoritative diagnostics
- `.assay/orchestrator/<run_id>/merge_report.json` — first place to look after any conflict resolution run; compare `resolutions[i].original_contents[j].content` (markers) vs `resolved_contents[j].content` (clean)
- `orchestrate_status` MCP tool — always includes `merge_report` key (null or object); `merge_report.resolutions` array length tells how many conflicts were auto-resolved
- `MergeReport.results[i].status` + `error` field — `ConflictSkipped` with error detail vs `Merged` with `merge_sha`; most complete per-session picture
- `tracing::warn` with `session_name` + `error` in `resolve_conflict()` — first look when a real Claude invocation fails; `raw_output_len` distinguishes empty vs garbage responses
- `MERGE_HEAD` in `.git/` after merge runner completes — indicates a cleanup bug in the abort path

### What assumptions changed
- M003-CONTEXT.md listed Codex/OpenCode adapters and SessionCore as M003 scope. Both were delivered in M002 (D024, D028) and deferred (D042) respectively before M003 began — M003 focused entirely on conflict resolution. The context document reflects the original planning assumptions, not the actual scope.
- T02 (S01) planned a `ConflictResolutionOutput` schema snapshot in assay-types. The type ended up crate-local in assay-core (D046), so only the `ConflictResolutionConfig` snapshot was needed. S02's audit type (`ConflictResolution`) correctly lives in assay-types with its snapshot.
- Validation failure return type: originally planned as `Err(...)` from the handler. The actual handler signature returns `ConflictResolutionResult`, so `Abort` is used for hard rollback failures — semantically equivalent but more explicit.

## Files Created/Modified

- `crates/assay-core/src/merge.rs` — abort_on_conflict parameter; two integration tests + shared helper
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — New module: ConflictResolutionResult, resolve_conflict(), run_validation_command(), prompt builders, 21 unit tests
- `crates/assay-core/src/orchestrate/merge_runner.rs` — conflict_resolution_enabled config, two-phase lifecycle, catch_unwind + SHA verify + abort cleanup, handler type -> ConflictResolutionResult, resolutions accumulation, repo_clean check
- `crates/assay-core/src/orchestrate/mod.rs` — pub mod conflict_resolver
- `crates/assay-core/tests/orchestrate_integration.rs` — test_merge_resolutions_audit_trail, test_merge_skip_leaves_empty_resolutions, create_branch_modifying_file helper; updated config constructions
- `crates/assay-types/src/orchestrate.rs` — ConflictResolutionConfig (with validation_command), ConflictFileContent, ConflictResolution, MergeReport.resolutions field; inventory entries; updated test initializers
- `crates/assay-types/src/lib.rs` — ConflictResolutionConfig, ConflictFileContent, ConflictResolution re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — conflict_resolution_config, conflict_file_content, conflict_resolution snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap` — locked (updated with validation_command)
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-file-content-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__merge-report-schema.snap` — regenerated (added resolutions property)
- `crates/assay-cli/src/commands/run.rs` — ConflictResolutionMode enum, parse_conflict_resolution, --conflict-resolution flag, handler composition, 4 tests
- `crates/assay-mcp/src/server.rs` — OrchestrateRunParams.conflict_resolution field, persist_merge_report(), OrchestrateStatusResponse wrapper, orchestrate_status extension, 4 new tests; tempfile dep
- `crates/assay-mcp/Cargo.toml` — tempfile.workspace = true added to [dependencies]
- `crates/assay-mcp/tests/mcp_handlers.rs` — updated orchestrate_status tests for new response shape; orchestrate_status_reads_merge_report_when_present
