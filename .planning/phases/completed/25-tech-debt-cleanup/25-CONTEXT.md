# Phase 25: Tech Debt Cleanup — Context

## Problem

During v0.2.0 development, two categories of tech debt accumulated:

### 1. Missing VERIFICATION.md (3 phases)

Phases 16 (Agent Gate Recording), 19 (Testing & Tooling), and 20 (Session JSONL Parser) were completed without formal VERIFICATION.md documents. Their summaries and UAT records confirm completion, but the standardized verification format is absent.

### 2. Open Issues (~128 files)

~128 issue files accumulated in `.planning/issues/open/` during development. These represent code quality observations, refactoring opportunities, and minor improvements. None are blockers.

## Scope

### Plan 25-01: Missing VERIFICATION.md Backfill

- Review completed phase summaries and UAT records for phases 16, 19, 20
- Create VERIFICATION.md for each with retroactive verification based on existing evidence
- Location: `.planning/phases/completed/{phase}/`

### Plan 25-02: Open Issues Triage

- Scan all open issue files
- Close issues that were resolved during subsequent phases
- Categorize remaining issues by priority and area
- Move resolved issues to `.planning/issues/closed/`
- Create TRIAGE-SUMMARY.md with remaining actionable items

## Decisions

### VERIFICATION.md Backfill

1. **Evidence sources**: Cross-reference git history AND existing docs (SUMMARY.md, UAT.md). Not docs alone — dig into git log for commit-level evidence.

2. **Template**: Use the exact same VERIFICATION.md template as other phases (e.g., Phase 23). No simplified retroactive format.

3. **UAT overlap**: Incorporate UAT results directly into VERIFICATION.md as part of the verification evidence. Single source of truth, not a reference link.

4. **Discrepancies**: If a verification criterion no longer holds in current code (renamed function, moved type), fix trivially if possible. File an issue for anything substantial.

5. **Build verification**: Confirm `just ready` passes on current code. Reference CI/merge commit evidence from git log for historical proof that it passed at merge time. Don't check out old commits.

6. **Granularity**: Match the pattern used by existing VERIFICATION.md docs — plan-level sections within a phase-level document (see Phase 23 as reference: requirement-level table + plan-level must-have tables).

### Issue Triage

7. **Closure bar**: If current code no longer exhibits the issue, close it with a note like "Resolved during Phase X" based on timeline correlation. No need for specific commit evidence.

8. **Noise filter**: Keep all valid observations open, regardless of how minor. No "won't fix" closures for trivial issues.

9. **Verification method**: Read the actual source code referenced by each issue to verify resolution. Don't just match against phase timelines.

10. **v0.1 issues**: Many v0.1-era issues (Phase 3, 6, 7, 8) are likely resolved by v0.2.0 refactoring (type relocation, evaluation rewrite, CLI hardening). Quick-scan these but still verify against current code.

### Issue Categorization

11. **Primary axis**: Priority — 3 tiers:
    - **must-fix**: Correctness issues, API design problems, missing validation
    - **should-fix**: Code quality, duplication, naming, ergonomics
    - **nice-to-have**: Doc comments, minor tests, cosmetic improvements

12. **Milestone target**: Each remaining issue gets tagged with a target milestone (v0.2.1, v0.3.0, or backlog).

13. **Disk layout**: Keep issues in place (flat `.planning/issues/open/` directory). Create a `TRIAGE-SUMMARY.md` that groups them by priority and area.

14. **Secondary grouping**: Within each priority tier, group by functional area (types, evaluation, history, MCP, CLI, context, guard, pruning).

## Deferred Ideas

None captured during discussion.
