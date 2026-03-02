# Phase 6: Spec Files — User Acceptance Tests

**Date:** 2026-03-02
**Result:** 7/7 PASS

| # | Test | Expected | Status |
|---|------|----------|--------|
| 1 | `assay spec show hello-world` | Table with spec name, criteria types colored | PASS |
| 2 | `assay spec show hello-world --json` | Valid pretty-printed JSON with criteria array | PASS |
| 3 | `assay spec show nonexistent` | Error message, exit code 1 | PASS |
| 4 | `assay spec list` | Lists hello-world with description | PASS |
| 5 | `assay spec list` outside project | Config not found error, exit code 1 | PASS |
| 6 | `NO_COLOR=1 assay spec show hello-world` | No ANSI escape codes in output | PASS |
| 7 | `just ready` | All tests pass (78), no warnings | PASS |
