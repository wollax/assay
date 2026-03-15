---
phase: 45-tech-debt-cleanup
plan: 02
status: complete
commits:
  - 5edd0c6 feat(45-02): Task 1 — missing derives, serde attrs, SessionPhase non_exhaustive
  - 79f3df5 feat(45-02): Task 2 — field type fixes, validation, doc fixes
issues_resolved: 14
---

# 45-02 Summary: assay-types v0.4.0 Sweep

## What Was Done

Applied type-level fixes across `assay-types` and related crates: missing derives, serde
attributes, field type corrections, a field rename with backward-compatible alias, a validation
rule, and doc fixes. All ~15 issues from the plan were resolved.

## Task 1: Missing Derives and Serde Attributes

**Derives added:**
- `CriterionOutcome` (evaluator.rs): added `Hash`
- `DiffTruncation` (gate_run.rs): added `Hash`
- `EvaluatorCriterionResult` (evaluator.rs): added `Hash` — all fields (String, enum) are Hash-compatible
- `EvaluatorSummary` (evaluator.rs): added `Hash` — no float fields, safe
- `PhaseTransition` (work_session.rs): added `Hash` — `DateTime<Utc>` implements Hash in chrono 0.4
- `RecoverySummary` (assay-core work_session.rs): added `Clone` and `PartialEq` (linter also added `Eq`)
- `SessionsConfig` (lib.rs): implemented `Default` manually returning `stale_threshold_secs: 3600` (not derived, since derive would use `u64::default() = 0`)

**Serde attributes:**
- `SessionPhase`: added `#[non_exhaustive]` — no match sites in the workspace use exhaustive match arms (only pattern uses like `matches!`, `!=`, `.is_terminal()`), so no wildcard additions were needed
- `GateEvaluateResponse.diff_truncation` (assay-mcp): added `#[serde(default)]` alongside existing `skip_serializing_if` — struct is Serialize-only currently, but guarded for future Deserialize additions
- `WorktreeInfo`, `WorktreeStatus`: added `#[serde(deny_unknown_fields)]`

**Rename:**
- `stale_threshold` → `stale_threshold_secs` with `#[serde(alias = "stale_threshold")]` for backward compatibility
- Updated all code references: `assay-mcp/src/server.rs`, `assay-core/src/config/mod.rs` tests

**Snapshots updated:** `config-schema.snap` (field rename), `worktree-status-schema.snap` (deny_unknown_fields → additionalProperties: false)

## Task 2: Field Type Fixes, Validation, Doc Fixes

**Type corrections:**
- `DiffTruncation.original_bytes` and `truncated_bytes`: `usize` → `u64` (platform-independent for serialization)
  - Construction site in `assay-mcp/src/server.rs`: added `as u64` casts on `.len()` calls
- `WorktreeStatus.ahead` and `behind`: `Option<usize>` → `Option<u32>`
  - Parse site in `assay-core/src/worktree.rs`: changed `parse::<usize>()` to `parse::<u32>()`

**Validation:**
- Added `stale_threshold_secs == 0` check to `config::validate()` in assay-core
- Returns `ConfigError { field: "[sessions].stale_threshold_secs", message: "must be a positive integer (greater than zero)" }`
- Tests added: `validate_rejects_zero_stale_threshold_secs`, `stale_threshold_secs_alias_backward_compat`

**Doc fixes:**
- `WorkSession.gate_runs`: added inline doc documenting ID format (`<timestamp>-<6-char-hex>`)
- `EvaluatorCriterionResult.name`: softened from "must match" to "expected to match, mismatches surface as warnings"
- `SessionsConfig.stale_threshold_secs`: updated phase reference from `agent_running` (snake_case) to `AgentRunning` (variant name), added "Must be greater than zero."

**Snapshots updated:** `diff-truncation-schema.snap` and `worktree-status-schema.snap` reflect `u64`/`u32` format changes.

## Issues Resolved (14)

Moved to `.planning/issues/closed/`:
- `criterion-outcome-missing-hash-derive`
- `diff-truncation-missing-hash-derive`
- `evaluator-result-types-missing-hash-derive`
- `phase-transition-hash-derive`
- `recovery-summary-missing-derives`
- `sessions-config-default-derive`
- `session-phase-non-exhaustive`
- `gate-evaluate-response-missing-serde-default`
- `diff-truncation-usize-vs-u64`
- `evaluator-criterion-result-name-doc-overstates`
- `gate-runs-id-format-doc`
- `sessions-config-doc-phase-ref`
- `stale-threshold-accepts-zero`
- `stale-threshold-secs-naming`

## Notes

- `worktree-missing-deny-unknown-fields` issue did not exist in the open issues list — the deny_unknown_fields addition was already covered by the plan requirement
- Concurrent execution with Plan 45-03 caused some issue files to be moved by both plans — the final state is correct with all issues in closed/
- `just ready` passes with no regressions (809 tests pass)
