# Phase 16 Verification

**Status:** passed
**Score:** 25/25 must-haves verified

## Must-Have Verification

### Plan 01 Must-Haves — Agent Gate Types and Criterion Extension

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | GateKind::AgentReport variant exists and serializes as 'AgentReport' | PASS | `crates/assay-types/src/gate.rs:32` — `AgentReport` variant in GateKind enum; test at `:283` verifies TOML contains `kind = "AgentReport"` |
| 2 | Criterion and GateCriterion have optional kind and prompt fields | PASS | `crates/assay-types/src/criterion.rs:65` — `pub kind: Option<CriterionKind>`; `:70` — `pub prompt: Option<String>`; `crates/assay-types/src/gates_spec.rs:47` — `pub kind: Option<CriterionKind>`; `:51` — `pub prompt: Option<String>` |
| 3 | EvaluatorRole enum serializes self/independent/human with SelfEval variant renamed to 'self' | PASS | `crates/assay-types/src/session.rs:23` — `pub enum EvaluatorRole`; test at `:124` verifies `SelfEval` serializes as `"self"` and roundtrips correctly |
| 4 | Confidence enum serializes as high/medium/low | PASS | `crates/assay-types/src/session.rs:43` — `pub enum Confidence` with `High`, `Medium`, `Low` variants; kebab-case serde attribute |
| 5 | GateResult carries optional agent evaluation fields (evidence, reasoning, confidence, evaluator_role) | PASS | `crates/assay-types/src/gate.rs:88` — `pub evidence: Option<String>`; `:93` — `pub reasoning: Option<String>`; `:98` — `pub confidence: Option<Confidence>`; `:103` — `pub evaluator_role: Option<EvaluatorRole>` |
| 6 | AgentSession type exists and is serializable for crash recovery | PASS | `crates/assay-types/src/session.rs:89` — `pub struct AgentSession` with Serialize/Deserialize derives; fields: session_id, spec_name, created_at, command_results, agent_evaluations (HashMap), criteria_names (HashSet), spec_enforcement (HashMap) |
| 7 | kind=AgentReport and cmd/path are mutually exclusive (validated at parse time) | PASS | `crates/assay-core/src/spec/mod.rs:130` — `// AgentReport criteria must not have cmd or path`; `:136` — error message "criterion has kind=AgentReport with `cmd`"; test at `:1863` — `AgentReport mutual exclusivity validation` |

### Plan 02 Must-Haves — Core Evaluation Dispatch and Validation

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Criteria with kind=AgentReport are skipped by evaluate_all/evaluate_all_gates with a pending marker | PASS | `crates/assay-core/src/gate/session.rs` referenced from `gate/mod.rs`; evaluate_all skips AgentReport criteria with result: None; test `evaluate_all_with_agent_criterion_marks_as_skipped` in gate/mod.rs |
| 2 | Spec validation rejects criteria that set kind=AgentReport with cmd or path | PASS | `crates/assay-core/src/spec/mod.rs:130-140` — validation block for AgentReport mutual exclusivity in `validate()`; `:398-408` — same block in `validate_gates_spec()`; test at `:1887` confirms rejection |
| 3 | Session lifecycle functions (create, report, finalize) exist in assay-core | PASS | `crates/assay-core/src/gate/session.rs:26` — `pub fn create_session`; `:51` — `pub fn report_evaluation`; `:103` — `pub fn finalize_session`; `:238` — `pub fn finalize_as_timed_out` |
| 4 | AssayError has session-related variants for invalid criterion, duplicate report, and session not found | PASS | `crates/assay-core/src/error.rs:136` — `SessionNotFound`; `:143` — `InvalidCriterion`; `:152` — `SessionError` |
| 5 | All downstream crates compile with the new type fields from Plan 01 | PASS | `just ready` passes (2026-03-07); all workspace crates compile cleanly |

### Plan 03 Must-Haves — MCP gate_report and gate_finalize Tools

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | MCP gate_report tool accepts agent-submitted pass/fail evaluations with structured reasoning | PASS | `crates/assay-mcp/src/server.rs:593` — `pub async fn gate_report`; `GateReportParams` at `:74` with session_id, criterion_name, passed, evidence, reasoning, confidence, evaluator_role fields |
| 2 | MCP gate_finalize tool produces a persisted GateRunRecord from accumulated session evaluations | PASS | `crates/assay-mcp/src/server.rs` — `gate_finalize` method calls `finalize_session()` which saves via `history::save()`; `GateFinalizeParams` at `:109` |
| 3 | gate_run auto-creates a session when spec contains AgentReport criteria and returns session_id | PASS | `crates/assay-mcp/src/server.rs:514` — `let session_id = session.session_id.clone()`; `:517-518` — sets `response.session_id` and `response.pending_criteria` |
| 4 | gate_report with an unknown criterion name returns a clear error | PASS | `crates/assay-mcp/src/server.rs:599` — checks `sessions.get_mut(&p.session_id)` with "Session not found" error; `report_evaluation` validates criterion name and returns `InvalidCriterion` |
| 5 | Agent-reported gates default to advisory enforcement unless spec overrides | PASS | `crates/assay-mcp/src/server.rs:208-211` — `advisory_passed`/`advisory_failed` counters tracked in response; enforcement resolution uses spec-defined level |
| 6 | Stale sessions auto-finalize on timeout (30 min) with partial results | PASS | `crates/assay-mcp/src/server.rs:305-306` — `const SESSION_TIMEOUT_SECS: u64 = 1800`; timeout task spawned in gate_run auto-finalizes via `finalize_as_timed_out` |
| 7 | Agent cannot escalate enforcement above spec-defined level | PASS | `crates/assay-core/src/gate/session.rs` — `finalize_session` resolves enforcement from spec-defined `spec_enforcement` map; agent evaluations cannot override the enforcement level stored in session |

### Plan 04 Must-Haves — Visual Distinction, Schema Snapshots, Quality Gate

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | GateKind::AgentReport criteria show [agent] label prefix in CLI gate run output | PASS | `crates/assay-cli/src/main.rs:465` — `GateKind::AgentReport => "[agent]"` |
| 2 | GateKind::Command criteria show [cmd] label prefix in CLI gate run output | PASS | `crates/assay-cli/src/main.rs:462` — `GateKind::Command { .. } => "[cmd]"` |
| 3 | GateKind::FileExists criteria show [file] label prefix in CLI gate run output | PASS | `crates/assay-cli/src/main.rs:463` — `GateKind::FileExists { .. } => "[file]"` |
| 4 | History detail view shows agent evaluation fields (evidence, reasoning, evaluator_role) | PASS | `crates/assay-cli/src/main.rs:1527` — displays `evaluator_role`; `:1530` — displays `confidence`; `:1533-1536` — displays `evidence` with 200-char truncation |
| 5 | JSON Schema snapshots are regenerated to include all new types | PASS | 25 snapshot files in `crates/assay-types/tests/snapshots/` including agent-evaluation, agent-session, confidence, criterion-kind, evaluator-role, gate-run-record schemas |
| 6 | just ready passes | PASS | `just ready` output: "All checks passed." (2026-03-07); 513 tests passed, 3 ignored |

## Quality Gate

- **`just ready`:** PASS (2026-03-07) — fmt-check ok, clippy ok, 513 tests passed (3 ignored), cargo-deny ok
- **Merge commit:** `1729e91` — PR #56 merged to main; CI passed at merge time

## Test Coverage Summary

Phase 16 test contributions:
- `crates/assay-types/src/session.rs` — 10 tests (EvaluatorRole serialization, Confidence roundtrip, AgentEvaluation, AgentSession)
- `crates/assay-types/src/gate.rs` — 4 tests (AgentReport TOML roundtrip, field skip/include, full roundtrip)
- `crates/assay-types/src/criterion.rs` — 4 tests (agent report roundtrip, kind omission, mixed spec)
- `crates/assay-core/src/gate/mod.rs` — 2 tests (evaluate agent criterion error, evaluate_all skips agent)
- `crates/assay-core/src/gate/session.rs` — 7 tests (create, report valid/invalid, multiple roles, finalize, priority, timeout)
- `crates/assay-core/src/spec/mod.rs` — 2 tests (AgentReport mutual exclusivity validation)
- `crates/assay-mcp/src/server.rs` — 8 tests (response serialization, kind_label, agent criteria extraction)
- `crates/assay-types/tests/schema_snapshots.rs` — 6 new snapshot tests (criterion-kind, evaluator-role, confidence, agent-evaluation, agent-session, gate-run-record)

## Gaps

None.
