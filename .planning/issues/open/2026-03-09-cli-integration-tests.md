# CLI Integration Tests

**Created:** 2026-03-09
**Source:** PR #75 review (Phase 26)
**Priority:** Medium
**Area:** CLI / Testing

## Description

After extracting the CLI monolith into `commands/` modules (Phase 26), the CLI crate has zero tests. While the extraction was a pure refactor, there is no safety net to catch wiring regressions.

## Suggested Approach

Add basic smoke tests using `assert_cmd` or similar:
1. `assay --help` exits 0 and contains expected subcommands
2. `assay init --help` exits 0
3. `assay gate --help` exits 0 and lists the `run` subcommand
4. `assay spec --help` exits 0

These would catch missing re-exports, broken dispatch, or accidental argument changes.

## Files

- `crates/assay-cli/src/commands/` — all modules
- `crates/assay-cli/tests/` — new test files needed
