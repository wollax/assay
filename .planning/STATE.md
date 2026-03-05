# State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-02)

**Core value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents
**Current focus:** v0.2.0 Dual-Track Gates & Hardening

## Current Position

Phase: 15 — Run History CLI (complete)
Plan: 02 of 2 (complete)
Status: Phase complete
Last activity: 2026-03-05 — Completed 15-02-PLAN.md

Progress: v0.2.0 [█████░    ] ~38%

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 100% (shipped) |
| v0.2.0 | 13 (11-23) | 52 | ~31% |

## Accumulated Context

### Decisions

v0.1.0 decisions archived to .planning/milestones/v0.1.0-ROADMAP.md

v0.2.0 decisions (from brainstorm + research):
- Agent gates receive evaluations via MCP, not call LLMs directly
- Self-evaluation + audit trail for v0.2; independent evaluator deferred to v0.3
- Keep core types domain-agnostic
- No built-in LLM client, no SpecProvider trait yet
- Pipeline semantics for future orchestrator design
- Type relocation (GateRunSummary -> assay-types) is highest-churn change — do first
- Agent-reported gates default to advisory enforcement (trust asymmetry)
- Per-spec subdirectory layout for results (.assay/results/{spec-name}/)
- Timestamp + 6-char random hex suffix for run IDs (no new crate)
- Include assay_version in GateRunRecord for future schema migration
- Two-tier enforcement only (required/advisory) — SonarQube validates no warnings tier
- Cozempic-inspired features (token diagnostics, team protection) added to v0.2.0 as phases 20-23
- Session JSONL parsing in Rust (not Python) — full feature parity with Cozempic, native performance
- Phases 20-23 are independent of 11-19 — can be worked in parallel or after gates
- Guard daemon uses kqueue (macOS) / inotify (Linux) for sub-second reactive recovery
- Pruning strategies compose sequentially, dry-run by default, team messages always protected

v0.2.0 decisions (from 11-01 execution):
- Clean break for type relocation: no re-exports from assay-core, all consumers import from assay_types
- Output types (GateRunSummary, CriterionResult) do NOT use deny_unknown_fields
- Schema registry entries added for both relocated types

v0.2.0 decisions (from 11-02 execution):
- Backward-compat test verifies GateRunSummary deserializes from minimal JSON without results field
- Skipped criterion test (result: None) verifies skip_serializing_if works correctly

v0.2.0 decisions (from 12-01 execution):
- cmd takes precedence over path when both set (simpler than mutual exclusivity validation)
- path field uses same serde attributes as cmd (skip_serializing_if + default)
- evaluate_file_exists implementation unchanged — already correct

v0.2.0 decisions (from 13-01 execution):
- Enforcement enum uses Copy trait (two-variant fieldless, read frequently during evaluation)
- Input types use Option<Enforcement> (None = inherit from gate section default); output types use concrete Enforcement
- GateSection uses deny_unknown_fields (user-authored input); EnforcementSummary does not (output type)

v0.2.0 decisions (from 13-02 execution):
- Backward compat preserved: passed/failed/skipped counts compute as before; EnforcementSummary is additive
- resolve_enforcement() is public for reuse by CLI/MCP pass/fail logic
- Validation enforces at-least-one-required at parse time, not evaluation time
- Descriptive-only criteria (no cmd/path) do not count as executable for the required check

v0.2.0 decisions (from 14-01 execution):
- GateRunRecord wraps GateRunSummary via summary field (no field duplication)
- spec_name accessed via record.summary.spec_name (already embedded, not duplicated at record level)
- save() takes assay_dir and derives results path internally
- No new error variants needed — existing AssayError::Io covers all history operations
- generate_run_id() is public for caller flexibility

v0.2.0 decisions (from 14-02 execution):
- PartialEq derived on GateRunRecord, GateRunSummary, CriterionResult, GateResult (non-breaking, enables structural equality assertions)

v0.2.0 decisions (from 15-01 execution):
- max_history defaults to None (no pruning); CLI will apply default_max_history() (1000) when absent
- Some(0) and None both skip pruning — zero is treated as unlimited
- prune() is private to the history module — only save() calls it
- SaveResult replaces PathBuf as save() return type

v0.2.0 decisions (from 15-02 execution):
- save_run_record() helper centralizes record construction and save logic
- Streaming mode records have empty results vec and zero total_duration_ms (no per-criterion timing)
- handle_gate_run_all() streaming path tracks per-spec counters via before/after delta

### Pending Issues

38 open issues (expanded from 30 after Phase 8-10 PR reviews)

### Blockers

None.

### Next Actions

Phase 15 complete. Next: Phase 16 or next pending phase.

### Session Continuity

Last session: 2026-03-05
Stopped at: Completed 15-02-PLAN.md (Phase 15 complete)
Resume file: None
