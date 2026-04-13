---
phase: 70-wire-resolution-preconditions
verified: 2026-04-13T18:00:00Z
status: passed
score: 9/9 must-haves verified
re_verification: false
---

# Phase 70: Wire Resolution Preconditions Verification Report

**Phase Goal:** `evaluate_all_gates()` calls `compose::resolve()` before evaluating criteria and `check_preconditions()` before evaluation, so gates with `extends`, `include`, and `[preconditions]` work correctly at runtime across all surfaces (CLI, MCP, TUI).
**Verified:** 2026-04-13T18:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth                                                                                               | Status     | Evidence                                                                                             |
|----|-----------------------------------------------------------------------------------------------------|------------|------------------------------------------------------------------------------------------------------|
| 1  | PreconditionFailed runs recorded in gate history with distinct marker                               | VERIFIED   | `save_blocked_run()` in `history/mod.rs` writes `precondition_blocked: Some(true)` to disk          |
| 2  | `last_gate_passed()` returns `Some(false)` for precondition-blocked records                         | VERIFIED   | Branch `if record.precondition_blocked == Some(true) { return Some(false); }` in `history/mod.rs`   |
| 3  | `PreconditionStatus::all_passed()` method exists with vacuous-truth semantics                       | VERIFIED   | `impl PreconditionStatus { pub fn all_passed(&self) -> bool { ... } }` in `precondition.rs`         |
| 4  | Old history records (without `precondition_blocked`) continue to deserialize correctly              | VERIFIED   | `#[serde(default, skip_serializing_if = "Option::is_none")]` on field + backward-compat test        |
| 5  | CLI `assay gate run` on Directory specs calls `compose::resolve()` then `check_preconditions()`    | VERIFIED   | 4 call sites of `compose::resolve` and `check_preconditions` in `gate.rs`                           |
| 6  | CLI streaming output shows `[Parent: slug]` / `[Library: slug]` source tags                        | VERIFIED   | `source_tag()` helper + `stream_criterion` updated to accept `source` param and render tag          |
| 7  | CLI returns exit code 2 for precondition failures; `--all` continues without stopping              | VERIFIED   | `return Ok(2)` after `!status.all_passed()` in `handle_gate_run`; `handle_gate_run_all` tracks `blocked_count` |
| 8  | MCP `gate_run` on Directory specs calls `compose::resolve()` then `check_preconditions()`          | VERIFIED   | `assay_core::spec::compose::resolve` and `assay_core::gate::check_preconditions` in `server.rs` spawn_blocking closure |
| 9  | MCP returns structured `outcome=precondition_failed` response (not error) for blocked specs        | VERIFIED   | `PreconditionFailedResponse { outcome: "precondition_failed", ... }` returned as `CallToolResult::success` |

**Score:** 9/9 truths verified

### Required Artifacts

| Artifact                                                  | Expected                                             | Status     | Details                                                                                    |
|-----------------------------------------------------------|------------------------------------------------------|------------|--------------------------------------------------------------------------------------------|
| `crates/assay-types/src/precondition.rs`                  | `PreconditionStatus::all_passed()` method             | VERIFIED   | Method present with 4 tests covering all edge cases including vacuous-truth empty status   |
| `crates/assay-types/src/gate_run.rs`                      | `GateRunRecord::precondition_blocked: Option<bool>`   | VERIFIED   | Field present with `serde(default, skip_serializing_if)`, 3 tests including backward-compat |
| `crates/assay-types/tests/snapshots/schema_snapshots__gate-run-record-schema.snap` | Updated schema snapshot | VERIFIED   | `"precondition_blocked"` key present in snapshot                                           |
| `crates/assay-core/src/history/mod.rs`                    | `last_gate_passed()` handles blocked records; `save_blocked_run()` | VERIFIED | Both functions present with 6 tests covering blocked/pass/fail/none cases |
| `crates/assay-cli/src/commands/gate.rs`                   | `compose::resolve` + `check_preconditions` + `evaluate_all_resolved` pipeline; `source_tag`; exit code 2 | VERIFIED | All 3 key functions called in both JSON and streaming paths; `source_tag` helper with 4 tests; 9 integration tests |
| `crates/assay-mcp/src/server.rs`                          | `compose::resolve` + `check_preconditions` pipeline; `CriterionSummary.source/source_detail`; `PreconditionFailedResponse` | VERIFIED | All wiring present; `source_fields` closure in `format_gate_response`; `PreconditionFailedResponse` struct; 10+ tests |
| `crates/assay-mcp/src/snapshots/assay_mcp__server__tests__gate_run_command_spec.snap` | Updated snapshot with `outcome` field | VERIFIED | `"outcome": "evaluated"` present in snapshot |

### Key Link Verification

| From                                         | To                                                     | Via                                                                 | Status  | Details                                                                       |
|----------------------------------------------|--------------------------------------------------------|---------------------------------------------------------------------|---------|-------------------------------------------------------------------------------|
| `crates/assay-core/src/history/mod.rs`       | `crates/assay-types/src/gate_run.rs`                   | `GateRunRecord.precondition_blocked` field                           | WIRED   | `record.precondition_blocked == Some(true)` read; `precondition_blocked: Some(true)` written |
| `crates/assay-cli/src/commands/gate.rs`      | `assay_core::spec::compose::resolve`                   | `compose::resolve()` called in handle_gate_run for Directory specs   | WIRED   | 4 call sites in gate.rs (JSON+streaming x handle_gate_run + handle_gate_run_all) |
| `crates/assay-cli/src/commands/gate.rs`      | `assay_core::gate::check_preconditions`                | `check_preconditions()` called before evaluation                     | WIRED   | 4 call sites in gate.rs matching the resolve call sites                       |
| `crates/assay-cli/src/commands/gate.rs`      | `assay_core::gate::evaluate_all_resolved`              | `evaluate_all_resolved()` replacing `evaluate_all_gates()` for Directory | WIRED | 2 call sites in gate.rs (JSON + streaming paths)                            |
| `crates/assay-mcp/src/server.rs`             | `assay_core::spec::compose::resolve`                   | `resolve()` in spawn_blocking closure for SpecEntry::Directory       | WIRED   | Call present at line ~1734                                                    |
| `crates/assay-mcp/src/server.rs`             | `assay_core::gate::check_preconditions`                | `check_preconditions()` after resolve, before evaluate               | WIRED   | Call present at line ~1753; returns `GateEvalOutcome::PreconditionFailed` if blocked |

### Requirements Coverage

| Requirement | Source Plan(s) | Description                                                                                  | Status     | Evidence                                                                               |
|-------------|----------------|----------------------------------------------------------------------------------------------|------------|----------------------------------------------------------------------------------------|
| PREC-03     | 70-01, 70-02, 70-03 | Precondition failures produce distinct `PreconditionFailed` result (blocked != failed) | SATISFIED  | `GateEvalOutcome::PreconditionFailed`, `PreconditionFailedResponse`, `precondition_blocked: Some(true)` in history |
| INHR-02     | 70-02, 70-03   | Extended gate inherits parent criteria with own-wins merge semantics                          | SATISFIED  | `compose::resolve()` called in both CLI and MCP paths; resolve handles own-wins merging |
| INHR-04     | 70-02, 70-03   | Gate run output shows per-criterion source annotation (parent vs own)                         | SATISFIED  | CLI: `source_tag()` formats `[Parent: slug]`/`[Library: slug]`; MCP: `CriterionSummary.source/source_detail` fields |
| CLIB-02     | 70-02, 70-03   | User can reference criteria libraries via `include` field in gate definitions                 | SATISFIED  | `compose::resolve()` handles `include` via `load_library` callback in both CLI and MCP |
| PREC-01     | 70-02, 70-03   | Gate skipped unless named spec's last gate run passed (`[preconditions].requires`)            | SATISFIED  | `check_preconditions()` queries `last_gate_passed()` for each requires slug             |
| PREC-02     | 70-02, 70-03   | Shell commands must succeed before gate evaluation (`[preconditions].commands`)               | SATISFIED  | `check_preconditions()` runs commands; blocked if any command fails                    |

All 6 requirements are mapped to Phase 70 in REQUIREMENTS.md and all are marked Complete. No orphaned requirements.

### Anti-Patterns Found

| File                                          | Line | Pattern                                      | Severity | Impact                                                                           |
|-----------------------------------------------|------|----------------------------------------------|----------|----------------------------------------------------------------------------------|
| `crates/assay-cli/src/commands/gate.rs`       | 291  | `TODO(M024/S02): stream_criterion event wiring` | Info   | Pre-existing TODO for a different milestone; unrelated to Phase 70 goal; no functional impact |

No blockers or warnings. The single TODO is pre-existing, scoped to a future milestone, and does not affect the Phase 70 goal.

### Human Verification Required

None. All key behaviors are verifiable via code inspection and tests:
- Pipeline wiring verified by grep (all 3 functions called in correct order)
- Source tag formatting verified by unit tests
- Exit code 2 verified by integration tests
- History persistence verified by integration tests
- MCP structured response verified by tests and snapshot

### Gaps Summary

No gaps. All must-haves verified, all 6 requirements satisfied, workspace compiles cleanly, all targeted tests pass (106 assay-types, 32 assay-core history, 10 assay-cli gate, 28 assay-mcp gate_run — all green).

---

_Verified: 2026-04-13T18:00:00Z_
_Verifier: Claude (kata-verifier)_
