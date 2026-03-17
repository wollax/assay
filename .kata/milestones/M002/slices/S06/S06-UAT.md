# S06: MCP Tools & End-to-End Integration — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: mixed (artifact-driven for MCP/CLI wiring + live-runtime for real agent orchestration)
- Why this mode is sufficient: The automated integration tests prove the full pipeline with real git repos and mock runners. Real agent invocation is the only remaining gap, which requires human-driven UAT.

## Preconditions

- `just ready` passes
- A project directory with at least one spec (e.g., from M001 UAT)
- For live-runtime tests: Claude Code CLI installed and authenticated

## Smoke Test

Run the integration tests to confirm the pipeline is healthy:
```bash
cargo test -p assay-core --features orchestrate --test orchestrate_integration
```
All 3 tests pass → basic pipeline is working.

## Test Cases

### 1. MCP orchestrate_status returns live state

1. Create a multi-session manifest and run it via `orchestrate_run` MCP tool
2. Note the `run_id` from the response
3. Call `orchestrate_status` with that `run_id`
4. **Expected:** Returns JSON with `phase`, `failure_policy`, and per-session entries showing correct states (Completed/Failed/Skipped)

### 2. CLI detects multi-session and routes to orchestrator

1. Create a TOML manifest with 2+ sessions, at least one with `depends_on`
2. Run `assay run manifest.toml --json`
3. **Expected:** Output includes `run_id`, `sessions` array with per-session outcomes, and `merge_report`. Not the single-session response format.

### 3. CLI single-session backward compatibility

1. Create a TOML manifest with exactly one session and no `depends_on`
2. Run `assay run manifest.toml`
3. **Expected:** Uses existing `run_manifest()` path. No orchestration phase markers on stderr. Same behavior as M001.

### 4. Real multi-agent orchestration (manual UAT)

1. Create 3 specs: `auth`, `api`, `tests` where `tests` depends on `auth` and `api`
2. Write a manifest:
   ```toml
   [[sessions]]
   name = "auth"
   spec = "auth"

   [[sessions]]
   name = "api"
   spec = "api"

   [[sessions]]
   name = "tests"
   spec = "tests"
   depends_on = ["auth", "api"]
   ```
3. Run `assay run manifest.toml --failure-policy skip-dependents --merge-strategy completion-time`
4. **Expected:** `auth` and `api` run concurrently, `tests` waits, all merge sequentially into base branch

## Edge Cases

### Failure propagation with --failure-policy abort

1. Create a manifest where the first session will fail (bad spec reference)
2. Run with `--failure-policy abort`
3. **Expected:** All remaining sessions cancelled (not just dependents). Exit code 1.

### Missing .assay/orchestrator gitignore

1. In a fresh project without `.assay/.gitignore`, run a multi-session manifest
2. **Expected:** Merge phase may fail with clean-worktree error. Error message should be actionable.

## Failure Signals

- `orchestrate_status` returns error for a run_id that should exist → state persistence broken
- Single-session manifest triggers orchestration phase markers → detection heuristic broken
- Integration tests fail with "untracked files" errors → .gitignore scaffolding missing
- Merge report shows 0 sessions merged when all succeeded → extract_completed_sessions broken

## Requirements Proved By This UAT

- R020 (Multi-agent orchestration) — automated integration tests prove DAG ordering, parallel execution, failure propagation, and merge. Manual UAT test case 4 proves real agent orchestration.
- R021 (Orchestration MCP tools) — test cases 1 proves programmatic access to orchestration state. orchestrate_run tested by integration tests.
- R023 (MergeRunner with sequential merge) — integration tests verify branches merge in correct order with MergeReport.

## Not Proven By This UAT

- Real Claude Code / Codex / OpenCode agent invocation in orchestrated runs (test case 4 is manual UAT, not automated)
- Per-session adapter selection (hardcoded to Claude Code per D040)
- Behavior under high concurrency (>8 sessions) — bounded concurrency is tested but not stressed
- Recovery from partial failures mid-merge (conflict handler is noop/skip default)
- `.assay/orchestrator/` gitignore auto-scaffolding (documented as follow-up, not implemented)

## Notes for Tester

- The integration tests use mock runners, not real agents. Test case 4 (real multi-agent) is the critical manual verification.
- Ensure `.assay/.gitignore` includes `orchestrator/` in any project where you run orchestrated sessions.
- The `--merge-strategy file-overlap` option is most useful when sessions modify overlapping files — use `completion-time` (default) for most cases.
- Exit code 2 specifically means merge conflicts were encountered.
