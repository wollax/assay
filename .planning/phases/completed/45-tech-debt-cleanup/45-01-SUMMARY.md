---
phase: 45-tech-debt-cleanup
plan: 01
subsystem: planning/issues
tags: [triage, backlog, won't-fix, deferred, no-code-change]
wave: 1
---

# Phase 45 Plan 01: Issue Triage and Backlog Reduction Summary

## Objective

Close 30 stale, superseded, or out-of-scope issues from `.planning/issues/open/` to reduce backlog noise before the code-change plans begin.

## Tasks Completed

| Task | Files Moved | Annotation | Commit |
|------|-------------|------------|--------|
| Task 1: Close superseded 2026-03-01 issues | 11 | Won't fix — superseded by v0.4.0 architecture (phases 35-44) | bb62b50 |
| Task 2: Defer guard daemon issues | 18 | Deferred — out of scope, coherent sub-sweep for dedicated guard cleanup phase | 8a33e38 |
| Task 2b: Close duplicate diagnostic Hash issue | 1 | Duplicate of `2026-03-11-diagnostic-derive-hash.md` — canonical issue retained | 8a33e38 |

**Total moved: 30 issues**

## Verification

- `.planning/issues/open/2026-03-01-*` — none remaining
- `.planning/issues/open/2026-03-07-guard-*` — none remaining
- `.planning/issues/open/2026-03-11-diagnostic-missing-hash-derive.md` — removed
- `.planning/issues/closed/` — 11 won't-fix + 18 deferred + 1 duplicate = 30 new entries

## Deviations

None. Plan executed exactly as specified — file moves only, no code changes.

## Files Modified

- `.planning/issues/closed/` — 30 new files (annotated copies of open issues)
- `.planning/issues/open/` — 11 won't-fix deletions + 18 guard deletions + 1 duplicate deletion

## Metrics

- Duration: ~5 minutes
- Issues moved: 30
- Code changes: 0
- Backlog reduction: ~272 → ~242 open issues
- Commits: 2 (bb62b50, 8a33e38)

## Notes

The `2026-03-01-*` issues predated the v0.4.0 architecture overhaul. The guard daemon issues form a coherent set that warrants its own dedicated cleanup phase rather than piecemeal fixes in this sweep. The `diagnostic-missing-hash-derive` issue was a duplicate already captured by `diagnostic-derive-hash` (addressed in Plan 04).
