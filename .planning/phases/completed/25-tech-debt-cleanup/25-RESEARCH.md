# Phase 25 Research: Tech Debt Cleanup

## VERIFICATION.md Template Pattern

**Confidence: HIGH** — Two existing examples analyzed (Phase 15, Phase 23).

### Template Structure

Two formats exist. The planner should use the Phase 23 format (more recent, more detailed):

```markdown
# Phase NN Verification

**Status:** passed
**Score:** X/Y must-haves verified

## Must-Have Verification

### [Requirement Group / Plan Name]

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| N | Description of must-have truth | PASS | `file.rs:line` — code-level evidence |

### Plan NN Must-Haves
(repeat per plan)

## Quality Gate
- `just ready`: **PASS** (fmt-check ok, clippy ok, N tests ok, cargo-deny ok)

## Test Coverage Summary
(list relevant test modules and counts)

## Gaps
None. (or list gaps)
```

### Key Differences Between Phase 15 and 23

| Aspect | Phase 15 | Phase 23 |
|--------|----------|----------|
| Evidence column | No (just Check + Status) | Yes (`file:line — description`) |
| Requirement grouping | By HIST-XX roadmap IDs + logical groups | By TPROT-XX roadmap IDs + by plan |
| Score format | `14/14 must-haves verified` | `32/32 must-haves verified` |
| Quality gate | Test count + `just ready` note | Same |
| Test coverage | Brief line | Detailed per-module breakdown |

**Decision for planner:** Use Phase 23 format with Evidence column. Per CONTEXT.md decision #1, evidence must come from git history AND existing docs, not just docs.

## Evidence Inventory

### Phase 16: Agent Gate Recording (4 plans, ~25 must-haves)

| Evidence Source | Available | Quality |
|-----------------|-----------|---------|
| 16-01-SUMMARY.md | Yes | Full: 3 tasks, commits, file lists, test counts |
| 16-02-SUMMARY.md | Yes | Full: 2 tasks, commits, deviations, test counts |
| 16-03-SUMMARY.md | Yes | Full: 2 tasks, commits, deviations |
| 16-04-SUMMARY.md | Yes | Full: 2 tasks, commits, deviations |
| 16-UAT.md | Yes | 6/6 tests passed, includes test details |
| 16-RESEARCH.md | Yes | Domain research |
| 16-CONTEXT.md | Yes | Phase decisions |
| Git merge commit | `1729e91` (PR #56) | Available for `just ready` evidence |
| Plan must-haves | 7 + 5 + 7 + 6 = 25 truths | All extractable from YAML frontmatter |

**Assessment:** Rich evidence. All 4 summaries contain commit hashes and specific code locations. UAT covers end-to-end scenarios. Must-have truths are explicitly listed in plan frontmatter. Verification is straightforward — read current source to confirm each truth still holds.

### Phase 19: Testing & Tooling (3 plans, ~13 must-haves)

| Evidence Source | Available | Quality |
|-----------------|-----------|---------|
| 19-01-SUMMARY.md | Yes | Compact: 1 task, verification commands |
| 19-02-SUMMARY.md | Yes | Full: 2 tasks, 19 new tests, 19 issues triaged |
| 19-03-SUMMARY.md | Yes | Full: 1 task, commit, verification commands |
| 19-UAT.md | Yes | 10/10 tests passed |
| 19-RESEARCH.md | Yes | Domain research |
| 19-CONTEXT.md | Yes | Phase decisions |
| Git merge commit | `68c6a4d` (PR #59) | Available |
| Plan must-haves | 3 + 5 + 5 = 13 truths | All extractable |

**Assessment:** Strong evidence. Phase 19 was heavily testing-focused, so verification should be mechanical — confirm deny.toml settings, test existence, dogfooding spec.

### Phase 20: Session JSONL Parser & Token Diagnostics (5 plans, ~34 must-haves)

| Evidence Source | Available | Quality |
|-----------------|-----------|---------|
| 20-01-SUMMARY.md | Yes | Full: 3 tasks, commits, 26 tests, type inventory |
| 20-02-SUMMARY.md | Yes | (not read but exists) |
| 20-03-SUMMARY.md | Yes | (not read but exists) |
| 20-04-SUMMARY.md | Yes | (not read but exists) |
| 20-05-SUMMARY.md | Yes | Full: quality gate, bug fix, 357 tests |
| 20-UAT.md | Yes | 10/10 tests passed |
| 20-RESEARCH.md | Yes | Domain research |
| 20-CONTEXT.md | Yes | Phase decisions |
| Git merge commit | `0406691` (PR #60) | Available |
| Plan must-haves | 7 + 11 + 7 + 5 + 4 = 34 truths | All extractable |

**Assessment:** Richest evidence set. 5 plans with detailed summaries. Phase 20 is the largest retroactive verification effort (~34 must-haves). The type-heavy nature of Plan 01 means many truths can be verified by checking type definitions still exist.

### Combined Must-Have Count

| Phase | Plans | Must-Haves | Complexity |
|-------|-------|------------|------------|
| 16 | 4 | 25 | Medium — types + core + MCP + CLI |
| 19 | 3 | 13 | Low — config + tests + dogfood spec |
| 20 | 5 | 34 | Medium — types + core + CLI + MCP |
| **Total** | **12** | **72** | |

## Open Issues Landscape

### Volume

**128 open issues** in `.planning/issues/open/`.

### Date Distribution

| Date | Count | Era | Notes |
|------|-------|-----|-------|
| 2026-03-01 | 22 | v0.1 | Early brainstorm/PR review issues |
| 2026-03-02 | 18 | v0.1 | MCP spike and Phase 8-10 review issues |
| 2026-03-04 | 40 | v0.2 transition | Phase 11-13 review issues (type system, enforcement) |
| 2026-03-05 | 27 | v0.2 | Phase 14-15 review issues (history, run records) |
| 2026-03-06 | 3 | v0.2 | Phase 19 review issues (testing/tooling) |
| 2026-03-07 | 18 | v0.2 | Phase 23 review issues (guard daemon) |

### Functional Area Distribution

| Area | Count | Notes |
|------|-------|-------|
| Guard (daemon, circuit breaker, thresholds) | 18 | All from Phase 23, likely still current |
| History (save, load, prune, records) | 11 | Mix of Phase 14-15 |
| Gate (evaluation, results, kinds) | 11 | Phase 12-13 era, may be resolved by later work |
| MCP (handlers, responses, tools) | 11 | Phase 8-10 era, partially resolved by Phase 19 |
| CLI (streaming, output, commands) | 9 | Phase 9 + 18 era |
| Spec (validation, parsing) | 7 | Phase 6 era, some may be resolved |
| GateRunRecord (serde, fields) | 4 | Phase 14-15 |
| Enforcement | 3 | Phase 13 |
| Testing/tooling | ~8 | Scattered (deny, self-check, coverage gaps) |
| Other (types, criterion, format, etc.) | ~46 | Various small items |

### Issue Structure

Issues follow a consistent format:
- YAML frontmatter: `created`, `title`, `area`, `severity` (and sometimes `provenance`, `files`)
- `## Problem` section
- `## Solution` section
- Most include specific file:line references

Severity distribution (from sampled issues):
- `important`: ~20-25%
- `suggestion`: ~50-60%
- `low`: ~15-20%
- Unmarked: ~5%

### Estimated Resolution Rates

**Confidence: MEDIUM** — Based on sampling ~15 issues and cross-referencing with phase work.

| Category | Count | Likely Resolved | Reasoning |
|----------|-------|-----------------|-----------|
| 2026-03-01 (v0.1) | 22 | ~12-15 (55-70%) | Many are about test gaps (filled by Phase 19), deny.toml (fixed by Phase 19), dogfood (done in 19-03), serde hygiene (addressed across v0.2) |
| 2026-03-02 (v0.1) | 18 | ~8-12 (45-65%) | MCP test coverage (filled by Phase 19), some MCP issues addressed by Phase 17 hardening |
| 2026-03-04 (v0.2) | 40 | ~5-10 (12-25%) | These are from Phase 11-13 reviews; most are structural suggestions unlikely to be addressed incidentally |
| 2026-03-05 (v0.2) | 27 | ~3-5 (11-18%) | Phase 14-15 review issues; most are design suggestions still relevant |
| 2026-03-06 (v0.2) | 3 | ~0-1 (0-33%) | Very recent, unlikely resolved |
| 2026-03-07 (v0.2) | 18 | ~0-2 (0-11%) | Most recent (Phase 23), almost certainly still current |
| **Total** | **128** | **~28-45** | **~22-35% estimated closure rate** |

**Key insight:** The 2026-03-01 and 2026-03-02 issues (40 total) have the highest expected closure rate because v0.2.0 refactoring (phases 11-24) likely addressed many of them. The verifier should prioritize these for quickest triage wins.

### Specific Known Resolutions

| Issue | Resolved By | Confidence |
|-------|-------------|------------|
| `deny-multiple-versions` | Phase 19-01 (cargo-deny tightening) | HIGH |
| `deny-source-controls` | Phase 19-01 | HIGH |
| `dogfood-checkpoint` | Phase 19-03 (dogfooding spec) | HIGH |
| `mcp-tool-handler-test-coverage` | Phase 19-02 (19 new tests) | HIGH |
| `test-coverage-gaps-phase3` | Phase 19-02 (test gap resolution) | HIGH — 19-02 explicitly triaged 19 issues |
| `test-coverage-gaps-phase6` | Phase 19-02 (partially) | MEDIUM |

## Workload Assessment

### Plan 25-01: VERIFICATION.md Backfill

| Item | Effort |
|------|--------|
| Read 12 plan files to extract must-haves | Medium (12 files, ~25-50 lines each) |
| Read 12 summary files for evidence | Medium (already done during research) |
| Cross-reference 72 must-haves against current source | High (need to read ~30-40 source files) |
| Read 3 UAT files and incorporate results | Low (already have content) |
| Verify `just ready` passes currently | Low (single command) |
| Check git log for merge commit evidence | Low (already identified: PRs #56, #59, #60) |
| Write 3 VERIFICATION.md files | Medium (template exists, fill in evidence) |

**Estimated total:** This is substantial but mechanical. The 72 must-haves are the bottleneck. Many will be verifiable by checking type/function existence. Recommend splitting into 3 sub-tasks (one per phase) within a single plan.

**Risk:** Phase 20 has 34 must-haves across 5 plans — this is the heaviest single verification. Phase 16 (25 must-haves) is moderate. Phase 19 (13 must-haves) is the lightest.

### Plan 25-02: Open Issues Triage

| Item | Effort |
|------|--------|
| Read all 128 issues | High (~60-80 files, many are short) |
| Verify resolved issues against source code | High (CONTEXT.md decision #9: read actual source) |
| Categorize by priority tier | Medium (apply severity heuristic) |
| Tag with target milestone | Low (mechanical once triaged) |
| Write TRIAGE-SUMMARY.md | Medium (summary document) |
| Close resolved issues (move or annotate) | Medium |

**Estimated total:** The 128-issue count is the main cost driver. Per CONTEXT.md decision #7, checking resolution requires source code reads. Per decision #9, this is non-negotiable.

**Optimization:** Start with 2026-03-01 and 2026-03-02 issues (40 total) where closure rate is highest. Then process by functional area to batch source code reads.

### Parallelization

25-01 and 25-02 are independent — they can be executed in parallel or in either order. However, doing 25-01 first may help 25-02 by deepening the verifier's understanding of what was actually built in each phase.

## Common Pitfalls

### For VERIFICATION.md Backfill (25-01)

1. **Stale line numbers.** Plan summaries reference specific line numbers (e.g., `session.rs:22`). These will have shifted since phases 16-20 due to later additions. Verify against current source, don't copy old line numbers.

2. **Must-have truth drift.** A truth like "Criterion has optional kind and prompt fields" may still be true but the field may have been renamed or the struct restructured. Check the actual current state.

3. **UAT test commands may have changed.** Phase 16 UAT references `assay spec list` — the current CLI may use `spec show` or different subcommands. Document the current command, not the UAT command.

4. **Over-relying on summaries.** Summaries say "all tests pass" but don't list which tests. The verifier must cross-reference against actual test modules.

5. **Missing roadmap requirement IDs.** Phase 23 VERIFICATION.md groups by TPROT-XX IDs from the roadmap. Phases 16, 19, 20 should use their own requirement IDs (AGNT-XX, TEST-XX/TOOL-XX, SDIAG-XX) if they exist in the roadmap. If not, group by plan number.

### For Issue Triage (25-02)

1. **False closure.** An issue about "SpecNotFound variant never constructed" might appear resolved if the variant was removed — but the actual problem (poor error messages for missing specs) may persist. Check the underlying problem, not just the specific symptom.

2. **Duplicate issues.** Several issues overlap (e.g., `history-io-error-conflation` and `history-serde-json-error-conflation`). Link duplicates in the triage summary.

3. **Phase 19 already triaged 19 issues.** Per 19-02-SUMMARY.md, Phase 19 "moved 19 issues to `.planning/issues/closed/`". The 128 remaining issues are post-Phase-19 or were not covered by that triage. Don't re-triage already-closed issues.

4. **Severity field inconsistency.** Some issues use `severity:` in frontmatter, others use `priority:`, some have neither. Normalize during triage.

5. **File references may be wrong.** Issues cite `crates/assay-cli/src/main.rs:653` but main.rs has changed significantly. The line reference is a hint, not an address.

## Recommendations

1. **Plan 25-01 structure:** One plan with 3 sub-sections (Phase 16, 19, 20). Phase 19 first (fewest must-haves, fastest win). Phase 20 last (most must-haves).

2. **Plan 25-02 structure:** One plan. Process issues in date order (oldest first, highest closure rate). Within each date group, batch by functional area to minimize file-switching overhead.

3. **Pre-flight check:** Run `just ready` once at the start to confirm the baseline is clean. Reference this single run for all three VERIFICATION.md quality gate sections, plus the merge commit from each phase's PR.

4. **Issue template for TRIAGE-SUMMARY.md:**
   ```markdown
   # Triage Summary

   **Date:** YYYY-MM-DD
   **Total issues:** 128
   **Closed as resolved:** N
   **Remaining:** M

   ## Priority Tiers

   ### Must-Fix (target: v0.2.1)
   | Issue | Area | Summary |
   ...

   ### Should-Fix (target: v0.3.0)
   ...

   ### Nice-to-Have (backlog)
   ...
   ```
