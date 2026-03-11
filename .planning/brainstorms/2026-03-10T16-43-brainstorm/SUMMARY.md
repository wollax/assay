# Brainstorm Summary: Completing the Orchestration Loop

**Date:** 2026-03-10
**Topic:** How should assay complete the core orchestration loop (discuss → research → plan → execute → verify/test → complete/merge)?
**Pairs:** 3 (planning primitives, merge-back workflow, dogfooding strategy)
**Rounds:** 2-3 per pair, all converged

---

## Key Finding

The orchestration loop is best closed incrementally across milestones, not in one shot. v0.4.0 stays unchanged (headless orchestration capstone). Near-term additions are small: one new MCP tool (`spec_create`) fits existing Phase 37, and two merge tools (`merge_check`, `merge_propose`) fit as a late v0.4.0 phase or v0.4.1 fast-follow. Full loop closure (planning primitives, spec composition, conflict resolution) targets v0.5.0+.

---

## Surviving Proposals by Category

### Planning Primitives ([Full report](planning-report.md))

| Proposal | Milestone | Scope | Summary |
|----------|-----------|-------|---------|
| `spec_create` MCP tool | v0.4.0 (Phase 37) | 1-2 days | Create specs from structured MCP params, validate before write |
| Requirement-level `depends_on` | v0.4.0 (Phase 37) | 1 day | Intra-spec dependency ordering + cycle detection |
| Criteria libraries (`include`) | v0.5.0 | 3-5 days | Reusable criteria from `.assay/criteria/`, design with `extends` |
| `spec_diff` git comparison | v0.5.0 | 3-5 days | Structural diff via `git show <ref>:path` |

**Killed:** `spec_update` (TOML round-trip problem + agents already do read-modify-write), `spec_decompose` (LLM IS the decomposition engine), `spec_from_issue` (LLM reads markdown better than regex heuristics).

**Key insight:** Agents handle decomposition, markdown parsing, and file manipulation natively. Assay provides *validation and structure*, not intelligence.

### Merge-Back Workflow ([Full report](mergeback-report.md))

| Proposal | Milestone | Scope | Summary |
|----------|-----------|-------|---------|
| `merge_check` (read-only conflict detection) | v0.4.0/v0.4.1 | 1-2 days | `git merge-tree --write-tree`, zero side effects |
| `merge_propose` (PR creation + gate evidence) | v0.4.0/v0.4.1 | 3-4 days | Push branch, create PR via `gh`, attach gate results |
| WorkSession merge states | v0.5.0 | — | After WorkSession stabilizes in production |
| `worktree_merge` (direct merge) | v0.5.0+ | — | Only if PR workflow proves insufficient |
| Conflict resolution strategies | v0.5.0+ | — | Auto/rebase/agent/human escalation ladder |

**Killed:** Auto-revert (data loss risk × flakiness), `MergeStrategy` config schema (YAGNI), multi-worktree ordering (GitHub merge queue handles this).

**Key insight:** PR creation is the universal merge pattern. `autonomous: false` maps naturally to "create PR with gate evidence, human reviews." Direct merge is deferred until there's evidence PR workflow is insufficient.

### Dogfooding Strategy ([Full report](dogfood-report.md))

| Recommendation | Timing | Summary |
|----------------|--------|---------|
| Keep v0.4.0 scope unchanged | Now | 11 phases well-scoped, Kata continues orchestrating |
| Lightweight dogfood experiment | During v0.4.0 | 2-3 specs + worktrees on safe phases (~4 hours), kill-switch retro |
| Identify external target project | During v0.4.0 | Design compass, not actual usage yet |
| Full dogfooding with gate_evaluate | v0.5.0 kickoff | External project + self-hosting after tooling matures |
| Kata → Assay transition design | v0.6.0+ | Open question requiring dedicated brainstorm |

**Dropped:** 11-phase spec mirroring (spec theater), worktree-per-phase as standard (bootstrap paradox), changing v0.4.0 scope.

**Key insight:** Dogfood quality gates now (research), dogfood orchestration later (v0.5.0+). Success metric is "design insights produced," not "bugs caught."

---

## Cross-Cutting Themes

1. **LLMs replace static tooling.** Three proposals were killed because agents already do the job better (decomposition, markdown parsing, issue-to-spec). Assay should provide structure and validation, not duplicate LLM capabilities.

2. **Read-only before read-write.** `merge_check` before `worktree_merge`, `spec_validate` before `spec_update`, `dry_run` on `merge_propose`. The codebase consistently benefits from read-only tools that build understanding before committing to side effects.

3. **Convention before infrastructure.** Criteria reuse via "baseline spec" convention (zero code) before `include` field. Hardcoded merge defaults before config schema. This pattern has worked well through v0.1.0-v0.3.0.

4. **PR-based workflow as the merge primitive.** The `autonomous: false` default makes PR creation the natural merge endpoint. Direct merge, conflict resolution, and merge queues are all deferred behind evidence of need.

5. **Sequenced dogfooding.** Gates now → full loop at v0.5.0 → orchestration at v0.6.0+. Each stage builds on the previous, with kill-switch checkpoints preventing sunk-cost continuation.

---

## Recommended Milestone Sequencing

| Milestone | Focus | Loop Coverage |
|-----------|-------|---------------|
| **v0.4.0** (current) | Headless orchestration + `spec_create` + intra-spec deps | Execute → Verify |
| **v0.4.1** (fast-follow) | `merge_check` + `merge_propose` | Complete/Merge |
| **v0.5.0** | Criteria libraries, `extends`, conflict resolution, full dogfooding | Discuss → Plan (partial) |
| **v0.6.0+** | Kata → Assay transition, planning primitives, orchestrator daemon | Full loop |

**Alternative:** If v0.4.0 has room, merge tools (`merge_check` + `merge_propose`) can be added as Phase 44.5 after WorkSession (Phase 40) ships.

---

## Impact on v0.4.0

| Change | Type | Effort |
|--------|------|--------|
| Add SPEC-05 (`spec_create`) to Phase 37 | +1 requirement | 1-2 days |
| Scope SPEC-04 to intra-spec only | Clarification | 0 days |
| Baseline spec convention (documented, no code) | Documentation | 0 days |

**Net v0.4.0 impact:** +1 requirement, no new phases, no scope risk.

---

*Synthesized from 3 explorer/challenger pairs, 19 proposals evaluated, 7 survived — 2026-03-10*
