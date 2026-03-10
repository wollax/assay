# Phase 31: Error Messages — UAT

**Phase:** 31-error-messages
**Started:** 2026-03-10
**Status:** Complete

## Tests

### ERR-01: Command-not-found shows actionable message

| # | Test | Expected | Status |
|---|------|----------|--------|
| 1 | Run a gate with a nonexistent command | Output contains "command 'X' not found. Is it installed and in PATH?" | PASS |
| 2 | Run a gate with a non-executable command (exit 126) | Output contains actionable permission message | PASS |

### ERR-02: Spec-not-found lists available specs

| # | Test | Expected | Status |
|---|------|----------|--------|
| 3 | Request a nonexistent spec when specs exist | Shows spec name and lists available spec names | PASS |
| 4 | Request a spec with a typo close to an existing name | Shows "Did you mean 'X'?" | PASS |
| 5 | Request a spec when no specs exist | Shows "No specs found in {dir}." | PASS |

### ERR-03: TOML parse error shows file path and line number

| # | Test | Expected | Status |
|---|------|----------|--------|
| 6 | Load a spec with invalid TOML syntax | Shows file path, line number, source line, and caret pointer | PASS |

## Results

**6/6 tests passed**
UAT complete.
