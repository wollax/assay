# Phase 12: FileExists Gate Wiring — UAT

## Test Results

| # | Test | Status | Notes |
|---|------|--------|-------|
| 1 | FileExists criterion (existing file) evaluates to passed: true | PASS | CLI: `file exists ... ok` |
| 2 | FileExists criterion (missing file) evaluates to passed: false with reason | PASS | CLI: `should fail ... FAILED`, stderr: `file not found: ...` |
| 3 | FileExists criteria are NOT skipped in evaluate_all | PASS | Mixed spec: 2 passed, 1 skipped (descriptive-only) |
| 4 | cmd takes precedence over path when both are set | PASS | stdout: `cmd-ran`, nonexistent path ignored |
| 5 | Existing specs without path field still parse (backward compat) | PASS | Legacy spec without path field runs correctly |
| 6 | Schema snapshots include new path field | PASS | Both criterion and gate-criterion snapshots contain `"path"` |

## Issues Found During UAT

### 1. CLI skip/count logic didn't recognize path-only criteria (FIXED)

- **Severity:** High
- **Description:** CLI layer had its own `executable_count` filter (`c.cmd.is_some()`) and `stream_criterion` skip logic that didn't account for the new `path` field. FileExists-only specs reported "No executable criteria found".
- **Fix:** Updated 4 locations in `crates/assay-cli/src/main.rs` to include `|| c.path.is_some()` / `&& criterion.path.is_none()`
- **Commit:** 141cdcd

## Summary

6/6 tests passed. One CLI-layer bug found and fixed during UAT.
