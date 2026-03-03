# Roadmap: Assay

## Milestones

<details>
<summary>✅ v0.1.0 Proof of Concept — SHIPPED 2026-03-02</summary>

**Goal:** Prove Assay's dual-track gate differentiator through a thin vertical slice — foundation types, spec-driven gates, MCP server, and Claude Code plugin.

- [x] Phase 1: Workspace Prerequisites (1 plan) — 2026-02-28
- [x] Phase 2: MCP Spike (1 plan) — 2026-02-28
- [x] Phase 3: Error Types and Domain Model (2 plans) — 2026-02-28
- [x] Phase 4: Schema Generation (1 plan) — 2026-02-28
- [x] Phase 5: Config and Initialization (3 plans) — 2026-03-01
- [x] Phase 6: Spec Files (2 plans) — 2026-03-01
- [x] Phase 7: Gate Evaluation (2 plans) — 2026-03-01
- [x] Phase 8: MCP Server Tools (2 plans) — 2026-03-01
- [x] Phase 9: CLI Surface Completion (2 plans) — 2026-03-02
- [x] Phase 10: Claude Code Plugin (2 plans) — 2026-03-02

[Full archive](milestones/v0.1.0-ROADMAP.md)

</details>

### 🔄 v0.2.0 Dual-Track Gates & Hardening

**Goal:** Ship agent-evaluated gates (via MCP `gate_report` tool), run history persistence, required/advisory gate enforcement, and comprehensive hardening of the v0.1 foundation.

- [ ] Phase 11: Type System Foundation
- [ ] Phase 12: FileExists Gate Wiring
- [ ] Phase 13: Enforcement Levels
- [ ] Phase 14: Run History Core
- [ ] Phase 15: Run History CLI
- [ ] Phase 16: Agent Gate Recording
- [ ] Phase 17: MCP Hardening & Agent History
- [ ] Phase 18: CLI Hardening & Enforcement Surface
- [ ] Phase 19: Testing & Tooling

---

## Progress Summary

| Milestone | Status | Phases | Requirements | Complete |
|-----------|--------|--------|--------------|----------|
| v0.1.0 Proof of Concept | Shipped | 10 | 43 | 100% |
| v0.2.0 Dual-Track Gates & Hardening | In Progress | 9 | 31 | 0% |
| v0.3.0 | Planned | — | — | — |

---

## v0.2.0 Phase Details

### Phase 11: Type System Foundation

**Goal:** Relocate result types from assay-core to assay-types and enforce serde hygiene across all domain types — the highest-churn refactor that must land before any feature work.

**Depends on:** None (first phase of v0.2.0)

**Requirements:**
- TYPE-01: `GateRunSummary` and `CriterionResult` relocated from assay-core to assay-types with `Deserialize` + `JsonSchema`
- TYPE-02: All domain types use `#[serde(skip_serializing_if)]` on optional fields
- TYPE-03: New fields use `#[serde(default)]` for backward compatibility with existing spec/config files

**Success Criteria:**
1. `GateRunSummary` and `CriterionResult` import from `assay_types` in all consuming crates and `just ready` passes
2. Serializing a type with `None` optional fields produces JSON without those keys
3. A v0.1.0-era spec file (without new fields) parses successfully under v0.2.0 code
4. JSON Schema snapshots reflect the relocated types with `Deserialize` + `JsonSchema` derives

---

### Phase 12: FileExists Gate Wiring

**Goal:** Connect the existing `FileExists` gate kind to the evaluation dispatch so it produces real results instead of being dead code.

**Depends on:** Phase 11

**Requirements:**
- TYPE-04: `FileExists` gate kind is wired into `evaluate()` dispatch (connect dead code)

**Success Criteria:**
1. A spec with a `FileExists` criterion pointing to an existing file evaluates to `passed: true` with evidence
2. A spec with a `FileExists` criterion pointing to a missing file evaluates to `passed: false` with a clear reason
3. `GateKind::FileExists` is no longer reachable as an unhandled match arm in gate evaluation

---

### Phase 13: Enforcement Levels

**Goal:** Add required/advisory enforcement to criteria so gate evaluation distinguishes blocking failures from informational warnings.

**Depends on:** Phase 11

**Requirements:**
- ENFC-01: Criterion has an `enforcement` field with values `required` (default) and `advisory`
- ENFC-02: Gate evaluation summary separates required failures from advisory failures

**Success Criteria:**
1. A criterion without an explicit `enforcement` field deserializes with `required` as the default
2. `evaluate_all()` returns a summary where required and advisory failure counts are reported separately
3. A gate with only advisory failures reports an overall `passed: true` status
4. A gate with any required failure reports `passed: false` regardless of advisory results

---

### Phase 14: Run History Core

**Goal:** Persist gate run results to disk as JSON files with atomic writes and retention policy, providing the audit trail infrastructure for all surfaces.

**Depends on:** Phase 11, Phase 13

**Requirements:**
- HIST-01: Gate run results are persisted to `.assay/results/<spec>/<timestamp>.json` after every evaluation
- HIST-04: Run history files use atomic write (tempfile-then-rename) to prevent corruption from concurrent writes

**Success Criteria:**
1. After `evaluate_all()`, a JSON file appears in `.assay/results/<spec-name>/` with the complete `GateRunRecord`
2. Two concurrent saves for the same spec produce two distinct files (no clobbering or corruption)
3. A partially-written file (simulated crash) does not leave a corrupt JSON file in the results directory
4. The persisted `GateRunRecord` deserializes back to the same logical content that was saved

---

### Phase 15: Run History CLI

**Goal:** Users can view gate run history from the command line, and results are automatically pruned to prevent unbounded accumulation.

**Depends on:** Phase 14

**Requirements:**
- HIST-02: User can view recent gate run history for a spec via `assay history <spec>`
- HIST-03: Run history has a configurable retention policy (per-spec file count limit) enforced on save

**Success Criteria:**
1. `assay history <spec>` displays a table of recent gate runs with timestamp, pass/fail, and criterion counts
2. When more than N results exist (N = configured retention limit), the oldest files are pruned on the next save
3. A user can configure the retention limit in `.assay/config.toml` and see it take effect on the next gate run

---

### Phase 16: Agent Gate Recording

**Goal:** Agents can submit gate evaluations via the MCP `gate_report` tool with structured reasoning, creating the second track of Assay's dual-track quality gates.

**Depends on:** Phase 13, Phase 14

**Requirements:**
- AGNT-01: MCP `gate_report` tool accepts agent-submitted pass/fail evaluations with structured reasoning
- AGNT-02: `GateKind::AgentReport` variant exists for criteria evaluated by agents (not shell commands)
- AGNT-03: Agent evaluations include `evaluator_role` metadata (`self`, `independent`, `human`)
- AGNT-04: Agent evaluation results are persisted to run history (same store as command gate results)

**Success Criteria:**
1. An agent calling `gate_report` with a spec name, criterion name, pass/fail, and reasoning receives a structured confirmation response
2. The submitted evaluation appears in the run history directory as a persisted `GateRunRecord`
3. Agent-reported results carry `evaluator_role` metadata distinguishing self-evaluation from independent evaluation
4. Agent-reported gates default to `advisory` enforcement unless explicitly overridden
5. `GateKind::AgentReport` criteria are visually distinct from `Command` criteria in gate summaries

---

### Phase 17: MCP Hardening & Agent History

**Goal:** Harden the MCP surface with timeout support, path validation, error handling, and documentation — then expose gate history to agents.

**Depends on:** Phase 14, Phase 16

**Requirements:**
- MCP-01: `gate_run` tool accepts a `timeout` parameter for agent-controlled timeouts
- MCP-02: `resolve_working_dir` validates that the path exists before evaluation
- MCP-03: `spec_list` handles scan errors gracefully instead of silently discarding them
- MCP-04: Tool descriptions are accurate and field-level documentation exists on response structs
- AGNT-05: MCP `gate_history` tool allows agents to query past gate run results for a spec
- ENFC-04: MCP `gate_run` response distinguishes required vs advisory results

**Success Criteria:**
1. An agent calling `gate_run` with a `timeout` parameter gets results within that timeout (or a timeout error)
2. An agent calling `gate_run` with a non-existent `working_dir` receives a clear error before any gate execution starts
3. An agent calling `gate_history` for a spec receives a list of recent run results with timestamps and outcomes
4. MCP `gate_run` responses include separate counts for required and advisory pass/fail results
5. `spec_list` returns partial results with error annotations when some spec files fail to parse

---

### Phase 18: CLI Hardening & Enforcement Surface

**Goal:** Polish the CLI with proper error propagation, exit codes, constants, and surface enforcement-level awareness in gate run output.

**Depends on:** Phase 13, Phase 15

**Requirements:**
- CLI-01: `main()` returns `Result` for proper error propagation
- CLI-02: Bare `assay` invocation exits with non-zero code
- CLI-03: `.assay` directory path is extracted to a named constant
- CLI-04: Gate command help duplication is resolved
- ENFC-03: CLI exit code reflects only required criterion failures; advisory failures are warnings

**Success Criteria:**
1. `assay` with no subcommand prints help and exits with a non-zero exit code
2. A gate run where all required criteria pass but advisory criteria fail exits with code 0 (success)
3. A gate run where any required criterion fails exits with a non-zero code regardless of advisory results
4. CLI error messages display the underlying cause chain (no silent swallowing of errors)
5. The `.assay` directory path appears as a named constant, not a string literal, in CLI code

---

### Phase 19: Testing & Tooling

**Goal:** Fill test coverage gaps, add comprehensive tests for all new v0.2.0 features, tighten cargo-deny policies, and dogfood Assay on itself.

**Depends on:** Phase 16, Phase 17, Phase 18

**Requirements:**
- TEST-01: MCP tool handlers have direct unit/integration tests (currently zero coverage)
- TEST-02: Test coverage gaps from Phase 3 and Phase 6 PR reviews are addressed
- TEST-03: All new features (history, enforcement, gate_report) have comprehensive tests
- TOOL-01: cargo-deny `multiple-versions` policy tightened from warn to deny
- TOOL-02: cargo-deny `source-controls` policy tightened from warn to deny
- TOOL-03: Dogfooding spec exists — Assay uses its own gates to enforce quality on itself

**Success Criteria:**
1. MCP tool handlers (`spec_get`, `spec_list`, `gate_run`, `gate_report`, `gate_history`) each have at least one direct test
2. `cargo deny check` passes with `multiple-versions` and `source-controls` set to `deny`
3. An `.assay/specs/self-check.toml` spec exists that runs Assay's own quality gates (fmt, clippy, tests, deny)
4. `just ready` passes and `assay gate run self-check` passes on a clean build

---

## Requirement Coverage

| Requirement | Phase | Description |
|-------------|-------|-------------|
| TYPE-01 | 11 | Relocate GateRunSummary/CriterionResult to assay-types |
| TYPE-02 | 11 | skip_serializing_if on optional fields |
| TYPE-03 | 11 | serde(default) for backward compat |
| TYPE-04 | 12 | Wire FileExists into evaluate() |
| ENFC-01 | 13 | Enforcement field on Criterion |
| ENFC-02 | 13 | Summary separates required/advisory |
| ENFC-03 | 18 | CLI exit code reflects required only |
| ENFC-04 | 17 | MCP response distinguishes required/advisory |
| HIST-01 | 14 | Persist results to .assay/results/ |
| HIST-02 | 15 | CLI history command |
| HIST-03 | 15 | Retention policy |
| HIST-04 | 14 | Atomic writes |
| AGNT-01 | 16 | gate_report MCP tool |
| AGNT-02 | 16 | GateKind::AgentReport variant |
| AGNT-03 | 16 | evaluator_role metadata |
| AGNT-04 | 16 | Agent results persisted to history |
| AGNT-05 | 17 | gate_history MCP tool |
| MCP-01 | 17 | gate_run timeout parameter |
| MCP-02 | 17 | Working dir validation |
| MCP-03 | 17 | spec_list error handling |
| MCP-04 | 17 | Tool description accuracy |
| CLI-01 | 18 | main() returns Result |
| CLI-02 | 18 | Bare invocation exit code |
| CLI-03 | 18 | .assay path constant |
| CLI-04 | 18 | Help duplication fix |
| TEST-01 | 19 | MCP handler tests |
| TEST-02 | 19 | Phase 3/6 review gaps |
| TEST-03 | 19 | New feature tests |
| TOOL-01 | 19 | cargo-deny multiple-versions → deny |
| TOOL-02 | 19 | cargo-deny source-controls → deny |
| TOOL-03 | 19 | Dogfooding spec |

**Coverage:** 31/31 requirements mapped (100%)
