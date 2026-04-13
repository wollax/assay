# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v0.7.0 — Gate Composability

**Shipped:** 2026-04-13
**Phases:** 8 | **Plans:** 19

### What Was Built
- Gate inheritance (`extends`) with own-wins merge semantics and per-criterion source annotations
- Criteria libraries in `.assay/criteria/` with load/save/scan API and `include` references
- Spec preconditions (`requires` + `commands`) with distinct `PreconditionFailed` result
- Guided wizard across CLI, MCP, and TUI — all delegating to shared `assay-core::wizard`
- Five new MCP tools for agent-driven composition (gate_wizard, criteria_list/get/create, spec_resolve)
- Composability validation in spec_validate (missing parents/libraries, cycles, path traversal)

### What Worked
- **Wizard-in-core pattern**: Building `apply_gate_wizard()` in core first meant CLI, MCP, and TUI surfaces were thin wrappers with zero duplicated validation logic
- **Milestone audit before completion**: The audit at 16:00 caught 6 unsatisfied requirements and 3 broken E2E flows. Phase 70 (gap closure) wired them in before shipping
- **TDD throughout**: resolve() tests written before implementation caught subtle own-wins merge edge cases
- **Parallel phase execution**: Phases 68/69 and 70/71 ran in parallel, saving wall-clock time

### What Was Inefficient
- **SUMMARY frontmatter gap**: None of 19 plan SUMMARYs include `requirements_completed` — a process consistency gap that forced audit to use 2-source instead of 3-source cross-reference
- **Phase 70 as gap closure**: The initial plan (phases 64-69) left runtime wiring incomplete. Phase 70 was needed to actually wire compose::resolve() into gate run paths — this should have been in the original plan
- **Phase 71 as gap closure**: TUI hardcoded specs_dir was caught by audit, not by phase 69 verification

### Patterns Established
- **Zero-trait convention preserved**: `resolve()` uses closures, not trait objects — consistent with 0-trait codebase (129K+ lines)
- **Exit code 2 for precondition-blocked**: Distinct from exit code 1 (gate failure) — agents and scripts can disambiguate
- **`Option<bool>` for backward-compat fields**: `precondition_blocked: Option<bool>` on GateRunRecord allows old history records to coexist with new ones

### Key Lessons
1. **Plan for wiring phases explicitly.** Type foundation + resolution core + evaluation integration are necessary but not sufficient — the runtime wiring into CLI/MCP paths needs its own phase, not an afterthought gap closure
2. **Audit before you ship, not after.** The milestone audit caught real gaps that would have shipped as silent broken features
3. **Pre-existing tech debt is acceptable.** The 2 partial E2E flows (milestone advance, PR check) bypass compose::resolve() but pre-date v0.7.0 — correctly classified as tech debt, not blockers

### Cost Observations
- Sessions: ~8 (across 3 days)
- Notable: 3-day turnaround for 8 phases, 19 plans, 24K insertions — audit + gap closure added ~6 hours but prevented shipping broken features

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change |
|-----------|--------|-------|------------|
| v0.1.0 | 10 | 18 | Initial vertical slice |
| v0.2.0 | 15 | 38 | Agent gates + context protection |
| v0.3.0 | 9 | 22 | Worktree foundation + polish |
| v0.4.0 | 11 | 30 | Headless evaluation capstone |
| v0.4.1 | 5 | — | Merge tools |
| v0.5.0 | 9 | — | Single-agent E2E pipeline |
| v0.6.0-6.2 | 4 | — | Multi-agent + P0 cleanup |
| v0.7.0 | 8 | 19 | Composability + wizard + audit-before-ship |

### Cumulative Quality

| Milestone | Tests | LOC (Rust) | Key Quality Gate |
|-----------|-------|------------|------------------|
| v0.1.0 | 119 | 5,028 | — |
| v0.2.0 | 493 | 23,385 | — |
| v0.3.0 | 603 | 27,067 | — |
| v0.4.0 | 836 | 33,462 | — |
| v0.7.0 | 2,505 | 129,826 | Milestone audit with gap closure |

### Top Lessons (Verified Across Milestones)

1. **Types first, wiring last.** Every milestone that starts with type foundation has smoother later phases — but runtime wiring needs explicit planning
2. **Milestone audits catch real gaps.** First use in v0.7.0 caught 6 unsatisfied requirements; will use for all future milestones
