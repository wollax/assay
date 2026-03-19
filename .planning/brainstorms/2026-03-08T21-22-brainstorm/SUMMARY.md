# Brainstorm Summary: Assay v0.3.0

**Date:** 2026-03-08
**Session:** 3 explorer/challenger pairs (quick wins, high value, radical)
**Rounds:** 2-3 rounds of debate per pair
**Duration:** ~15 minutes

---

## Quick Wins — Low-effort, high-impact improvements

6 proposals survived debate, ~8 days total effort.

| # | Proposal | Effort | Impact | Issues Resolved |
|---|----------|--------|--------|-----------------|
| 1 | CLI Correctness Sprint | 1.5d | High | ~8-10 CLI bugs |
| 2 | MCP Parameter Validation | 1-1.5d | High | 2-3 MCP issues |
| 3 | Types Hygiene Tier A | 0.5d | Medium | ~10-12 type issues |
| 4 | Gate/Spec Error Messages | 1d | Medium | 3-4 error issues |
| 5 | Gate Output Truncation | 3-4d | High | Streaming capture |
| 6 | Guard Daemon PID fsync | 0.25d | Low | 1 correctness fix |

**Key debate outcomes:**
- `assay doctor` killed → replaced with targeted error messages
- Types hygiene split into zero-risk (Tier A) and needs-analysis (Tier B)
- Guard daemon narrowed from 15 issues to 1 correctness fix
- MCP fuzzy matching killed → parameter structure validation instead

[Full report](quickwins-report.md)

---

## High Value — Substantial features worth investment

5 features accepted, 2 killed, ~8.5-9.5 weeks total.

| # | Feature | Scope | Dependencies | Risk |
|---|---------|-------|-------------|------|
| 1 | Worktree Manager | 2 wks | None | Medium |
| 2 | Claude Code Launcher | 1.5 wks | Worktree | Low-Med |
| 3 | Session Record | 1 wk | Worktree | Low |
| 4 | Gate Evaluate | 3 wks | Worktree + Session | High |
| 5 | Minimal TUI | 1 wk | Session + Gates | Low |

**Killed:**
- Merge-Back Pipeline (premature without orchestrator)
- Spec Provider Trait (premature abstraction, one implementation)

**Critical insight:** v0.3.0 targets a **headless sequential workflow** (agent runs once → gates run after → human reviews), NOT the full MCP-integrated iterative workflow. Setting this expectation correctly is essential.

**Sequencing:**
```
Week 1-2:   Worktree Manager
Week 2-3:   Claude Code Launcher + Session Record (parallel)
Week 4-6:   Gate Evaluate
Week 7:     Minimal TUI
Week 8-9:   Integration + Polish
```

[Full report](highvalue-report.md)

---

## Radical — New directions and paradigm shifts

3 ideas explored, each with an actionable seed for v0.3.0.

| # | Idea | Core Insight | v0.3.0 Seed | Seed Effort |
|---|------|-------------|-------------|-------------|
| 1 | Gate Marketplace | Gate definitions are shareable TOML | `[gate.extends]` composable criteria | 2d |
| 2 | Trust Scores | Assay already collects all raw quality data | `gate history --summary` pass rates | 1d |
| 3 | Spec-as-Contract | Specs should validate both human and agent sides | `[preconditions]` in specs | 1-2d |

**Cross-cutting theme:** Assay's unique position is structured quality data. All three radical directions are aggregation/composition layers on top of data the tool already captures.

[Full report](radical-report.md)

---

## Deferred / Dropped Items

| Item | Reason | Revisit When |
|------|--------|-------------|
| Merge-Back Pipeline | Premature without orchestrator | v0.4 after workflow proven |
| Spec Provider Trait | One implementation = premature abstraction | Second provider materializes |
| `assay doctor` command | New feature, not quick win | After error messages improve |
| Types Hygiene Tier B | Serde changes need compatibility analysis | After stored history format review |
| Guard daemon batch (14 issues) | Low-usage feature, narrow to correctness only | User demand |
| Full TUI Dashboard | Requires orchestrator for real-time multi-session | v0.4 |

---

## Recommended v0.3.0 Scope

Combining quick wins + high-value features + radical seeds:

**Phase 1: Foundation (weeks 1-2)**
- Quick wins: Types Hygiene A, CLI Correctness, Guard fsync
- High value: Worktree Manager

**Phase 2: Core Features (weeks 3-5)**
- Quick wins: MCP Validation, Error Messages
- High value: Claude Code Launcher + Session Record (parallel)

**Phase 3: Differentiation (weeks 6-8)**
- Quick win: Gate Output Truncation
- High value: Gate Evaluate (independent evaluation)

**Phase 4: Polish (weeks 9-10)**
- High value: Minimal TUI
- Radical seeds: `[preconditions]`, `gate history --summary`, `[gate.extends]`
- Integration testing + end-to-end workflow validation

**Total estimate:** ~10 weeks combining all tracks

---

*Synthesized from 3 brainstorm pairs — 2026-03-08*
