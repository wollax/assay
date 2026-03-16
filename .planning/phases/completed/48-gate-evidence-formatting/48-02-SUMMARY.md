---
phase: 48
plan: 02
status: complete
started: 2026-03-16T21:40Z
completed: 2026-03-16T21:50Z
---

# Plan 02 Summary: Gate Evidence Formatting Tests

## Tasks Completed

| # | Task | Commit | Status |
|---|------|--------|--------|
| 1 | Formatting and detail section tests | `427ab43` | Done |
| 2 | Truncation and persistence tests | `427ab43` | Done |

Tasks 1 and 2 were implemented together in a single commit since all tests go in the same `#[cfg(test)] mod tests` block.

## What Was Done

Added 15 new tests to `crates/assay-core/src/gate/evidence.rs`:

**Helpers added:** `make_file_exists_pass`, `make_file_exists_fail`, `make_skipped`

**Formatting tests:**
- `all_pass_deterministic_has_no_detail_sections` — 3 deterministic passes, no `<details>` sections
- `mixed_results_produce_correct_structure` — command pass/fail, agent pass/fail, skipped: verifies emoji counts, bold on failures, detail section presence/absence
- `detail_sections_have_blank_lines_for_github_rendering` — blank line after `<summary>` and before `</details>`
- `enforcement_summary_shows_breakdown` — enforcement line with required/advisory breakdown
- `file_exists_fail_detail_section_via_helper` — FileExists fail shows missing path
- `file_exists_pass_has_no_detail_section` — FileExists pass is table-only

**Truncation tests:**
- `no_truncation_within_limit` — full output under limit, `pr_body == full_report`
- `truncation_removes_failures_last` — agent passes removed before failures
- `truncation_preserves_summary_table` — summary table survives aggressive truncation
- `full_report_is_never_truncated` — `full_report` identical regardless of `char_limit`
- `truncation_with_multibyte_utf8_uses_byte_length` — byte-length enforcement with multi-byte chars

**Persistence tests:**
- `save_report_creates_directories` — creates `reports/<spec>` directory if missing
- `save_report_content_matches_full_report` — saved content matches `full_report`
- `save_report_rejects_slash_in_run_id` — path traversal via run_id rejected

## Deviations

1. **Clippy fix:** `map_or(false, ...)` changed to `is_some_and(...)` per `clippy::unnecessary_map_or` lint.
2. **Emoji counts adjusted:** `mixed_results` test counts include detail section summary emojis, not just table emojis.

## Decisions

- Combined Task 1 and Task 2 into a single commit since all tests share the same file and test module.

## Verification

`just ready` passes: fmt-check, clippy, all 614 tests, cargo-deny.
