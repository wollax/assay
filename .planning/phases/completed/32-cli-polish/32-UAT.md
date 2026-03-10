# Phase 32: CLI Polish — UAT

## Tests

| # | Test | Status |
|---|------|--------|
| 1 | NO_COLOR disables color output | PASS |
| 2 | Piped output disables colors (TTY detection) | PASS |
| 3 | `assay gate --help` shows subcommand list only (no examples) | PASS |
| 4 | `assay gate run --help` shows full examples | PASS |
| 5 | `[srs]` magic string uses constant (no hardcoded literals) | PASS |
| 6 | Gate history table renders correctly | PASS |

## Results

**6/6 tests passed**

### Test Details

1. **NO_COLOR=1 disables colors** — Verified with `NO_COLOR=1 assay spec list | cat -v`. No ANSI escape codes in output.
2. **Piped output disables colors** — Verified `assay gate run self-check 2>&1 | cat -v` shows no `^[[32m` codes. Confirmed colors present when running under `script` (TTY wrapper).
3. **Gate top-level help** — `assay gate --help` shows only "Commands: run, history, help" with no Examples block.
4. **Gate run subcommand help** — `assay gate run --help` shows full examples section (6 examples).
5. **No hardcoded [srs]** — `grep` for `"[srs]"` in `crates/assay-cli/src/` returns 0 matches. All 3 sites use `assay_types::DIRECTORY_SPEC_INDICATOR`.
6. **History table** — `assay gate history self-check --limit 3` renders aligned columns with pass/fail status, timestamps, and duration.

**Date:** 2026-03-10
