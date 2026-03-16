---
phase: 48-gate-evidence-formatting
plan: 01
subsystem: gate-evidence
tags: [markdown, formatting, truncation, pr-body, github]
requires: []
provides: [FormattedEvidence type, format_gate_evidence(), save_report(), GITHUB_BODY_LIMIT]
affects: [50-merge-propose]
tech-stack:
  added: []
  patterns: [semantic-truncation, section-priority-removal, std::fmt::Write string building]
key-files:
  created:
    - crates/assay-types/src/evidence.rs
    - crates/assay-core/src/gate/evidence.rs
  modified:
    - crates/assay-types/src/lib.rs
    - crates/assay-core/src/gate/mod.rs
decisions:
  - "Detail sections use <details open> for failures, <details> (collapsed) for agent passes, no sections for deterministic passes"
  - "Truncation removes sections by priority: agent passes first, then failures, then table rows as last resort"
  - "Report files written to .assay/reports/<spec-name>/<run-id>.md via simple std::fs::write (non-atomic)"
  - "Enforcement breakdown shown as inline text, not a separate table"
  - "Emoji mapping: :white_check_mark: pass, :x: fail, :fast_forward: skip"
metrics:
  duration: 452s
  completed: 2026-03-16
---

# Phase 48 Plan 01: Gate Evidence Formatting Summary

Markdown renderer that transforms GateRunRecord into PR-body-ready markdown with semantic truncation at section boundaries using byte-length checks, plus disk persistence of full untruncated reports.

## What Was Built

### FormattedEvidence type (assay-types)
- `pr_body: String` — truncated markdown for PR body
- `full_report: String` — full untruncated markdown for disk
- `truncated: bool` — whether truncation was applied
- Registered in schema registry via `inventory::submit!`

### format_gate_evidence() (assay-core::gate::evidence)
- Accepts `&GateRunRecord`, `&Path` (report path), `usize` (char limit)
- Produces structured markdown: H2 header with spec name, summary stats, enforcement breakdown, status table with emoji, detail sections
- Failed criteria: `<details open>` with command output, exit codes, or agent reasoning
- Agent-evaluated passes: collapsed `<details>` with evidence/reasoning/confidence
- Deterministic passes: table-only, no detail section
- Blank line after `<summary>` tag for GitHub rendering compatibility
- Footer with run ID, timestamp, assay version

### Semantic truncation
- Builds full report, then progressively removes sections to fit limit
- Priority order: agent passes removed first, then failures, then table rows
- Uses `.len()` (byte count) for all size checks — not `.chars().count()`
- Appends truncation notice with report path when truncated
- `GITHUB_BODY_LIMIT` constant exported as 65,536

### save_report()
- Writes full report to `.assay/reports/<spec-name>/<run-id>.md`
- Validates path components via `validate_path_component` (reused from history module)
- Creates directories as needed, returns written path

## Tests

19 tests covering:
- Empty results produce valid markdown
- Header, table, emoji, bold failures
- Detail section open/collapsed behavior
- Blank line after summary (GitHub pitfall)
- Footer metadata
- Truncation priority ordering
- Byte-length truncation
- Truncation notice with report path
- FileExists failure rendering
- Duration formatting
- save_report file creation and path traversal rejection

## Deviations from Plan

None — plan executed exactly as written.

## Commits

| Hash | Message |
|------|---------|
| edcb7be | feat(48-01): define FormattedEvidence type in assay-types |
| 4eee96f | feat(48-01): implement format_gate_evidence with semantic truncation and save_report |
| 7d8aa75 | style(48-01): fix formatting and clippy warnings in evidence module |

## Next Phase Readiness

Phase 48-02 (tests plan) can proceed — all public APIs are implemented and verified.
Phase 50 (Merge Propose) can consume `format_gate_evidence()` and `save_report()` directly.
