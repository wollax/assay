# M003: Conflict Resolution & Polish

**Vision:** When multi-agent orchestration produces merge conflicts, an AI evaluator automatically resolves them — with an audit trail and optional post-resolution validation — making the merge pipeline fully autonomous.

## Success Criteria

- A multi-session orchestration with overlapping file changes produces a merge conflict that is automatically resolved by an AI evaluator, producing a clean merge commit
- The conflict handler receives a live conflicted working tree (not an already-aborted one) and can stage, resolve, and commit
- Resolution details (original markers, resolved content, rationale) are recorded in MergeReport for post-hoc inspection
- An optional validation command (e.g., `cargo check`) runs after AI resolution and before accepting the merge commit — failing validation rejects the resolution
- CLI `--conflict-resolution` flag and MCP `orchestrate_run` parameter control whether AI resolution is active
- All existing merge runner tests continue to pass (default behavior unchanged: auto-abort + skip)

## Key Risks / Unknowns

- **Two-phase merge lifecycle** — `merge_execute()` currently auto-aborts on conflict. Changing to leave the repo in a conflicted state requires careful error handling (partial resolution, handler crash) to avoid leaving dirty repos.
- **Sync evaluator in sync merge loop** — `run_evaluator()` is async (tokio), but the merge runner runs in `std::thread::scope`. Need a sync subprocess path for conflict resolution.
- **AI resolution quality** — The evaluator may produce subtly wrong merges (duplicate definitions, broken imports). Post-resolution validation mitigates but doesn't eliminate this risk.

## Proof Strategy

- Two-phase merge lifecycle → retire in S01 by modifying `merge_execute()` to optionally preserve conflict state and proving a custom handler can resolve + commit through a real git repo integration test
- Sync evaluator → retire in S01 by using `std::process::Command` (sync) directly for the conflict resolution subprocess, bypassing the async `run_evaluator()` path
- AI resolution quality → retire in S02 by adding configurable post-resolution validation that runs a command and rejects the merge if it fails

## Verification Classes

- Contract verification: unit tests for two-phase merge, conflict prompt construction, resolution parsing, audit trail serialization; insta snapshots for new types; integration tests with real git repos and scripted resolvers
- Integration verification: end-to-end test through `merge_completed_sessions()` with a conflict handler that invokes a real subprocess to resolve conflicts; CLI integration test routing `--conflict-resolution auto` through the full orchestration pipeline
- Operational verification: none beyond M002's requirements
- UAT / human verification: real Claude invocation resolving a genuine merge conflict in a multi-session orchestration

## Milestone Definition of Done

This milestone is complete only when all are true:

- All slice deliverables are complete and `just ready` passes
- `merge_execute()` supports two-phase conflict lifecycle (attempt → handler → commit/abort) with rollback on handler failure
- A concrete AI conflict handler closure is wired into both CLI and MCP orchestration paths
- MergeReport includes resolution audit details (before/after, rationale) for every resolved conflict
- Post-resolution validation command is configurable and exercised in tests
- Integration test proves: multi-session manifest → overlapping changes → conflict → AI resolution → clean merge → audit trail in report
- Success criteria are re-checked against live behavior via integration tests with real git repos

## Requirement Coverage

- Covers: R026 (AI conflict resolution)
- Partially covers: none
- Leaves for later: R025 (SessionCore — deferred per D042, cost/benefit unfavorable), R027 (OTel — deferred per D030 to M004+)
- Orphan risks: none — all Active requirements are either validated or explicitly deferred with rationale

## Slices

- [x] **S01: AI Conflict Resolution** `risk:high` `depends:[]`
  > After this: `merge_completed_sessions()` with an AI conflict handler resolves real git merge conflicts — proven by integration test with a scripted resolver subprocess against a real git repo. CLI `--conflict-resolution auto` flag routes to the handler. Existing merge behavior (auto-abort + skip) is unchanged by default.
- [x] **S02: Audit Trail, Validation & End-to-End** `risk:medium` `depends:[S01]`
  > After this: MergeReport includes full resolution audit trail (original markers, resolved diff, resolver output). Post-resolution validation command rejects bad merges. `orchestrate_status` MCP tool shows resolution details. End-to-end CLI integration test proves the assembled pipeline: multi-session manifest → conflict → AI resolution → validation → audit trail.

## Boundary Map

### S01 → S02

Produces:
- `merge_execute_two_phase()` or modified `merge_execute()` that optionally leaves repo in conflict state, returning `MergeExecuteResult` with conflict details while working tree is still conflicted
- `resolve_conflict()` sync function: reads conflicted files, builds prompt from `ConflictScan` + file contents, spawns `claude` subprocess (sync `std::process::Command`), parses resolution, applies to files, stages + commits, returns `ConflictAction::Resolved(sha)`
- `ConflictResolutionConfig` type controlling: enabled/disabled, model, validation command, timeout
- Updated `merge_completed_sessions()` that manages two-phase lifecycle: on conflict with resolution enabled, calls handler with live conflicted tree; on handler failure, runs `git merge --abort`; on handler success, verifies commit SHA
- CLI `--conflict-resolution auto|skip` flag on `assay run`
- MCP `orchestrate_run` tool accepts `conflict_resolution` parameter
- New error variants for resolution failures (handler crash, validation failure, abort failure)
- Schema snapshots for any new/modified types

Consumes:
- nothing (first slice)

### S02 (terminal)

Produces:
- `ConflictResolution` type recording: session name, conflicting files, original markers, resolved content, resolver stdout, validation result
- `MergeReport.resolutions: Vec<ConflictResolution>` field with resolution audit details
- `validation_command` field on `ConflictResolutionConfig` — runs after resolution, rejects merge on non-zero exit
- Extended `orchestrate_status` MCP tool response with resolution details
- End-to-end CLI integration test: manifest → orchestrate → conflict → resolve → validate → report with audit trail
- Schema snapshots for `ConflictResolution` and updated `MergeReport`

Consumes:
- Two-phase `merge_execute()` with live conflict state from S01
- `resolve_conflict()` function and `ConflictResolutionConfig` from S01
- Updated `merge_completed_sessions()` conflict lifecycle from S01
