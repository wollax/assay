# Research Summary: Assay v0.2.0 "Dual-Track Gates & Hardening"

**Date:** 2026-03-02
**Synthesized from:** STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md
**Milestone scope:** Run History, Required/Advisory Gates, Agent Gate Recording, Foundation Hardening

---

## Executive Summary

Four parallel research streams converge on a consistent picture: the v0.2.0 feature set is well-scoped, implementable without new dependencies, and architecturally sound. The highest-risk change is not a new feature — it is a type relocation (`GateRunSummary` and `CriterionResult` from `assay-core::gate` to `assay-types`) required by run history persistence. The second-highest concern is `deny_unknown_fields` on user-facing types, which needs a deliberate resolution strategy for forward compatibility. Everything else is additive and low-risk.

The research validates the brainstorm scope entirely. No features need to be cut; no dependencies need to be added; no architectural rewrites are required.

---

## Key Findings by Area

### 1. Stack (zero new dependencies)

**Finding:** All v0.2.0 features are implementable with the existing workspace dependency set.

The only concrete change is promoting `serde_json` from `[dev-dependencies]` to `[dependencies]` in `assay-core/Cargo.toml`. The full JSON persistence stack (`serde_json`, `chrono`, `std::fs`) is already present. `rmcp` 0.17.0 (released 2026-02-27) is the current version and requires no upgrade. The `gate_report` tool is a straightforward addition to the existing `#[tool_router]` impl block.

Rejected without regret: NDJSON libraries, UUID crates, file-locking crates, embedded databases (SQLite, sled, redb). All were evaluated and found to be complexity without benefit at Assay's scale.

**One opportunity:** rmcp 0.17.0 introduces `Json<T>` structured output. Use it for `gate_report` since it returns structured data agents need to parse. Migrating existing tools is optional and non-blocking for v0.2.

**Concrete `Cargo.toml` diff for `assay-core`:**
```diff
 [dependencies]
 assay-types.workspace = true
 chrono.workspace = true
 serde.workspace = true
+serde_json.workspace = true
 thiserror.workspace = true
 toml.workspace = true
```

**Confidence:** High (all answers verified against codebase, crates.io, and rmcp 0.17.0 release notes).

---

### 2. Features (industry-validated scope)

**Finding:** The two-tier enforcement model and file-per-run history are industry-proven patterns. The `gate_report` tool is genuinely novel in the MCP ecosystem.

**Run History:** The industry split is clear — local-first tools (nextest, dbt, agentic-orchestration) use JSON/NDJSON files; server tools (SonarQube) use databases. Assay is local-first. JSON files in `.assay/results/` is the correct choice. File-per-run is simpler than NDJSON append logs for the scale (<1000 results per project) and avoids concurrent-write complexity.

**Enforcement Levels:** SonarQube's history confirms the lesson: three-tier enforcement (pass/warning/fail) creates warning fatigue and was removed in v7.6. GitLab CI's two-tier model (`allow_failure`) is the proven UX. Assay ships `required` (default) and `advisory`, nothing more. Default must be `required` to preserve backward compatibility and prevent "everything is optional" drift.

**Agent Gate Recording:** No MCP tool in the ecosystem currently implements a `gate_report` submission pattern. The closest analogs (agentic-orchestration critic loops, MCPx-eval judges) are integrated rather than externalized. Assay's `gate_report` creates a new pattern: externalized quality assessment with structured evidence, submitted by the agent that did the work. This is the v0.2 differentiator.

**Hardening:** The existing open issues inventory (20 items) provides the full hardening backlog. The credibility of a quality tool depends entirely on its own quality. Hardening ships alongside new features, not deferred after.

**Recommended feature ordering:**
1. Hardening prerequisites (fix issues that cause churn if done after feature work)
2. Enforcement levels (small type change, big behavioral impact; prerequisite for agent evaluation)
3. Run history (depends on enforcement levels for correct status recording)
4. Agent gate recording (depends on run history for persistence infrastructure)

---

### 3. Architecture (clear integration path, one medium-risk refactor)

**Finding:** The v0.2.0 features integrate cleanly into the existing architecture. The dependency graph requires no changes. All new code follows established patterns. One type relocation is necessary and safe but touches multiple files.

**New components:**
- `assay-core::history` module — `save()`, `list()`, `list_recent()`, `load()`, `latest()`
- `assay-mcp` tools — `gate_report` and `gate_history`
- `assay-cli` subcommand — `assay gate history <spec>`

**New types in `assay-types`:**
- `Enforcement` enum (Required/Advisory) on `Criterion`
- `GateRunSummary` and `CriterionResult` — moved from `assay-core::gate`, gains `Deserialize + JsonSchema`
- `GateRunRecord` — persistence wrapper with trigger and metadata
- `RunTrigger` enum (Cli, Mcp, Agent)
- `GateKind::AgentReport` variant

**The type relocation (`GateRunSummary` → `assay-types`) is the highest-churn change.** It touches `assay-core::gate::evaluate_all()`, `assay-mcp::server::format_gate_response()`, MCP server tests, and `assay-cli::main.rs`. This is a compile-error-driven refactor — move the types, fix imports until `just ready` passes. It is safe and necessary; keeping `GateRunRecord` in `assay-types` while `GateRunSummary` remains in `assay-core` would create a circular dependency.

**CLI streaming → history integration:** The CLI uses per-criterion streaming (`stream_criterion()`), not `evaluate_all()`. To persist run history without losing the streaming UX, accumulate `CriterionResult` entries alongside the streaming display, then construct and save a `GateRunRecord` at the end.

**`.assay/results/` gitignore:** Already present in `init.rs::render_gitignore()`. History files will be gitignored by default without any new work.

**MCP tool count:** v0.1.0 has 3 tools. v0.2.0 targets 5 (`gate_report`, `gate_history` added). Stay at 5 maximum to bound context window cost for agents listing tools.

**History module API (assay-core::history):**
```rust
pub fn save(root: &Path, record: &GateRunRecord) -> Result<PathBuf>
pub fn list(root: &Path, spec_name: &str) -> Result<Vec<GateRunRecord>>
pub fn list_recent(root: &Path, spec_name: &str, limit: usize) -> Result<Vec<GateRunRecord>>
pub fn load(root: &Path, spec_name: &str, run_id: &str) -> Result<GateRunRecord>
pub fn latest(root: &Path, spec_name: &str) -> Result<Option<GateRunRecord>>
```

---

### 4. Pitfalls (17 identified, 4 critical)

**Finding:** The four critical pitfalls each have clear preventions. None are blockers; all are design decisions to make early.

**P-21 (Critical): `deny_unknown_fields` blocks forward compatibility**

`Spec`, `Criterion`, `Config`, and `GatesConfig` all use `#[serde(deny_unknown_fields)]`. Adding new fields in v0.2 (e.g., `enforcement` on `Criterion`) means a spec file that includes the new field cannot be parsed by v0.1 Assay. This is a downgrade scenario — but a real UX concern for projects that share spec files across tool versions.

Prevention: Add all new fields with `#[serde(default, skip_serializing_if = "...")]` so they are omitted from serialized output when at default values, preserving round-trip compatibility. Do not remove `deny_unknown_fields` — it catches real typos. Write explicit backward-compatibility tests: parse v0.1.0-era spec files with v0.2 code. No whitelist mechanism exists in serde (issue #1864).

**P-22 (Critical): Concurrent result file writes**

Two concurrent gate runs on the same spec can corrupt result files. Prevention: use atomic writes via `tempfile::NamedTempFile::persist()` (crate already in workspace deps). The unique-filename-per-run strategy (timestamp + short random suffix) eliminates contention without requiring file locking.

**P-23 (Critical): Unbounded result file accumulation**

Agent retry loops can generate thousands of result files. Prevention: implement a per-spec retention policy at write time — keep only the last N files (default: 20). The existing 64KB `MAX_OUTPUT_BYTES` truncation already limits per-file size. Implement retention on day one, not as an afterthought.

**P-24 (Critical): Agent self-grading bias**

The same agent that implemented code evaluating whether it meets criteria has documented biases (anchoring, sycophancy, criterion reinterpretation). Prevention: require `reasoning` field (already planned), add `confidence` field, default agent gates to `advisory` enforcement, and never allow an agent gate result to override a failed command gate.

**P-35 (Integration): Result shape divergence between persistence and MCP API**

`GateRunResponse` (MCP projection) and the persisted `GateRunRecord` must not diverge. Prevention: define a single canonical type in `assay-types` (`GateRunRecord`) and project from it: `GateRunSummary → GateRunRecord` on persist, `GateRunRecord → GateRunResponse` on serve.

**P-36 (Integration): Trust mismatch in required agent gates**

A `required + AgentEval` gate has the same blocking power as `required + Command` but far lower reliability. A command gate produces deterministic, reproducible results. An agent gate's pass/fail is probabilistic. Prevention: default agent-reported gates to `advisory` enforcement regardless of spec configuration; display agent gate results with visual distinction in CLI and MCP responses.

---

## Confidence Levels

| Area | Confidence | Basis |
|------|-----------|-------|
| Zero new dependencies needed | High | Verified against codebase, crates.io, rmcp 0.17.0 release notes |
| File-per-run history design | High | Industry comparison (nextest, dbt, agentic-orchestration) |
| Two-tier enforcement correctness | High | SonarQube removal of warning tier; GitLab CI `allow_failure` pattern |
| `gate_report` MCP tool novelty | High | No equivalent found in MCP ecosystem survey |
| Type relocation churn estimate | High | All consumers identified and mapped in ARCHITECTURE.md |
| `deny_unknown_fields` forward-compat impact | High | Verified against serde issue tracker (issue #1864) |
| Agent self-grading bias severity | High | Multiple AI evaluation research sources (2025) |
| Retention policy urgency | Medium | Depends on usage patterns; agent loops are the high-risk scenario |

---

## Gaps and Open Questions

**Gap 1: `assay_version` in persisted records**
PITFALLS.md (P-26) recommends including `assay_version` in `GateRunRecord` for future schema migration. STACK.md and ARCHITECTURE.md do not include it in their proposed schemas. Decision needed before finalizing `GateRunRecord` type. Recommendation: include it; one field, high future value.

**Gap 2: Result file directory layout**
STACK.md proposes flat (`.assay/results/{spec-name}_{timestamp}.json`). ARCHITECTURE.md proposes per-spec subdirectory (`.assay/results/{spec-name}/{timestamp}.json`). FEATURES.md uses `.assay/runs/` as the root. Recommendation: ARCHITECTURE.md's per-spec subdirectory layout — bounds per-directory file counts, makes per-spec queries faster, and is consistent with per-spec retention pruning.

**Gap 3: Run ID generation strategy**
STACK.md prefers timestamp + random suffix (no new crate). FEATURES.md proposes ULID or UUID v7 (time-ordered, adds a crate). Recommendation: timestamp + 6-char random hex suffix, no new crate. Stable run IDs for cross-referencing are out of scope for v0.2.

**Gap 4: `gate_history` vs `gate_report` naming alignment**
ARCHITECTURE.md uses `gate_history` for the history query tool. STACK.md uses `gate_report` for agent submission with a separate history tool implied. Confirm final tool names before implementation to avoid SKILL.md churn.

---

## Roadmap Implications

### Must-do before any feature work

1. Audit all `AssayError` match sites (`rg 'AssayError::' crates/`) before touching error types
2. Resolve Gaps 2 and 3 above (directory layout and run ID strategy)
3. Close stale issues #19-23 (reference `spike.rs` which was replaced in v0.1)

### Phase ordering (validated by research)

**Phase 1: Hardening prerequisite issues**
Fix issues that cause churn if deferred: #33 (SpecNotFound construction), #30 (failure reason from stdout), #37 (validate working_dir), #12 (TUI try_init), #31 (gate_run timeout param). Small, independent, unblock clean feature work.

**Phase 2: Type foundation**
Add `Enforcement` enum; add `enforcement` field to `Criterion` with `#[serde(default)]`; move `GateRunSummary` and `CriterionResult` to `assay-types`; add `Deserialize + JsonSchema`; add `GateKind::AgentReport`; add `GateRunRecord` and `RunTrigger`; update schema snapshots. Run `just ready`.

**Phase 3: Core logic**
Update `evaluate_all()` for enforcement-aware counting; implement `assay-core::history` module with retention policy built in; add `HistoryIo`, `HistoryParse`, `CriterionNotFound` error variants. Run `just ready`.

**Phase 4: MCP surface**
Wire history saving into `gate_run`; update `GateRunResponse`/`CriterionSummary` for enforcement; add `gate_report` tool (advisory-by-default for agent-reported gates); add `gate_history` tool; add timeout parameter (#31); fix remaining MCP hardening issues (#32, #34, #35, #36, #38). Run `just ready`.

**Phase 5: CLI surface**
Collect `CriterionResult` entries during streaming; save `GateRunRecord` after all criteria evaluated; add `assay gate history <spec>` subcommand; update `print_gate_summary()` for advisory counts; fix #13 (CLI Result return type). Run `just ready`.

**Phase 6: CI/Build hardening**
Fix #14 (schema validation in CI), #15 and #16 (deny.toml tightening). Final `just ready`.

### Anti-features (confirmed out of scope)

| Anti-feature | Rationale |
|---|---|
| SQLite or embedded database | Overkill for local file-based tool |
| NDJSON with compression | Premature at this scale |
| Three-tier enforcement | SonarQube removed warnings in v7.6; warning fatigue is real |
| `prompt` field on `Criterion` | LLM API dependency; `gate_report` is the v0.2 mechanism |
| Automated re-evaluation triggers | Agent decides when to re-evaluate |
| Multi-agent consensus evaluations | v0.3+ complexity |
| Workflow state machine | Assay is a quality tool, not an orchestrator |
| Auto-retry on gate failure | Orchestration is agtx's job |
| Run diffing | Agent context eviction makes delta refs unreliable |
| Error telemetry / phone-home | Local tool, no data collection |

---

## Critical Decisions Required Before Implementation

| Decision | Options | Recommendation |
|----------|---------|----------------|
| Result file directory layout | Flat vs per-spec subdirectory | Per-spec subdirectory (`.assay/results/{spec-name}/`) |
| Run ID strategy | Timestamp+random suffix vs ULID vs UUID v7 | Timestamp + 6-char random hex (no new crate) |
| `assay_version` in `GateRunRecord` | Include vs omit | Include; cost is one field, value is future schema migration |
| Agent gate enforcement default | Advisory vs Required | Advisory by default; explicit override via flag |
| `GateRunSummary` location | Keep in core (add Deserialize) vs move to types | Move to `assay-types`; cleanest long-term architecture |
| `gate_report` output format | Text content vs `Json<T>` structured | `Json<T>` via rmcp 0.17.0 for new tools; retrofit existing tools later |

---

*Synthesized from parallel research by 4 agents — 2026-03-02*
