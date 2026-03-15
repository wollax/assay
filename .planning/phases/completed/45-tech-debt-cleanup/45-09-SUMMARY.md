---
plan: 45-09
status: complete
wave: 3
issues_resolved: 15
---

# Plan 45-09 Summary: MCP Doc Fixes, spec_get Correctness, Tests

## What Changed

### crates/assay-mcp/src/server.rs

**Doc fixes (Task 1a–1e):**
- Removed hardcoded tool count ("eighteen") from module-level doc; replaced with "tools" (no count).
- Clarified `GateReportParams.session_id` description: explicitly distinguishes the in-memory AgentSession ID returned by `gate_run` from the persisted WorkSession ID created by `session_create`.
- Cleaned up `GateReportResponse.warnings` doc comment: replaced stale implementation note with a user-facing description.
- Verified `GateHistoryListResponse.total_runs` doc was already updated by Plan 08 (raw file enumeration semantics).
- Expanded `get_info` instructions to clearly differentiate gate workflow (gate_run for immediate results) from session workflow (session_create for long-running tracked work).

**Code correctness (Task 1f–1j):**
- Verified `first_nonempty_line` already used `.trim().is_empty()` correctly — no change needed.
- Fixed `spec_get` to surface feature spec load errors: added `feature_spec_error` field to directory spec response instead of silently returning `feature_spec: null` on parse failure.
- Removed unnecessary `.clone()` calls in `spec_get` resolved block insertion by switching from `match &entry` to `match entry` (move semantics).
- Removed duplicate doc comment from `SpecGetParams.resolve` field (kept only `#[schemars(description = ...)]`).
- Added `load_config` project validation to `worktree_list` handler, matching the pattern used by `worktree_create`, `worktree_status`, and `worktree_cleanup`.

**Compilation fix (from Plan 08 partial apply):**
- Fixed `EvaluateCriterionResult` construction: removed string-mapping match arms (`"pass"`, `"fail"`, etc.) since the struct now uses `assay_types::CriterionOutcome` and `assay_types::Enforcement` directly.

**Test improvements (Task 2):**
- Extracted `single_failing_summary(criterion_name, stdout, stderr)` helper to reduce boilerplate in failure-reason tests.
- Added `test_failure_reason_stdout_multiline_uses_first_nonempty_line`: verifies only the first non-empty line of multiline stdout is used as the failure reason.
- Added `test_failure_reason_stdout_skips_leading_empty_lines`: verifies leading empty lines are skipped.
- Added `spec_get_resolve_true_directory_format_returns_resolved_block`: integration test verifying the `resolved` block appears correctly for directory-format specs.
- Pinned 5 disjunctive test assertions:
  - `test_load_spec_entry_not_found`: pinned to `"No specs found"`
  - `spec_get_missing_spec_returns_error`: pinned to `"No specs found"`
  - `gate_run_nonexistent_spec_returns_error`: pinned to `"No specs found"`
  - `gate_run_nonexistent_working_dir_returns_error`: pinned to exact error message
  - `session_update_not_found`: pinned to `"not found"`

## Issues Resolved (15)

| Issue | Description |
|-------|-------------|
| `server-module-doc-tool-count` | Module doc hardcoded tool count |
| `tool-count-in-docs-fragile` | Same (duplicate tracking) |
| `gate-report-session-id-ambiguous` | session_id doc unclear |
| `gate-report-warnings-comment-noise` | Stale comment in warnings field |
| `get-info-session-vs-gate-workflow-clarity` | get_info instructions unclear |
| `first-nonempty-line-whitespace` | Already correct; closed |
| `spec-get-silent-feature-spec-error` | Feature spec errors swallowed |
| `mcp-unnecessary-clones` | Unnecessary .clone() in resolved block |
| `spec-get-resolve-duplicate-description` | Duplicate doc + schemars description |
| `mcp-tests-no-specs-disjunction` | Disjunctive test assertions |
| `multiline-stdout-fallback-test` | Missing test for multiline stdout |
| `failure-reason-test-helper` | No helper for failure-reason tests |
| `spec-get-resolve-directory-format-test` | No test for directory spec + resolve |
| `worktree-list-mcp-no-project-check` | worktree_list skipped project validation |

Note: `gate-history-total-runs-doc-unclear` was already resolved by Plan 08.

## Verification

`just ready` passes: fmt-check + clippy + all tests (115 assay-mcp, 557 assay-core, plus others) + cargo-deny.
