# Phase 25: Tech Debt Cleanup

## Problem

During v0.2.0 development, two categories of tech debt accumulated:

### 1. Missing VERIFICATION.md (3 phases)

Phases 16 (Agent Gate Recording), 19 (Testing & Tooling), and 20 (Session JSONL Parser) were completed without formal VERIFICATION.md documents. Their summaries and UAT records confirm completion, but the standardized verification format is absent.

### 2. Open Issues (128 files)

128 issue files accumulated in `.planning/issues/open/` during development. These represent code quality observations, refactoring opportunities, and minor improvements. None are blockers.

## Scope

### Plan 25-01: Missing VERIFICATION.md Backfill

- Review completed phase summaries and UAT records for phases 16, 19, 20
- Create VERIFICATION.md for each with retroactive verification based on existing evidence
- Location: `.planning/phases/completed/{phase}/`

### Plan 25-02: Open Issues Triage

- Scan all 128 open issue files
- Close issues that were resolved during subsequent phases
- Categorize remaining issues by area/priority
- Move resolved issues to `.planning/issues/closed/`
- Summary of remaining actionable items
