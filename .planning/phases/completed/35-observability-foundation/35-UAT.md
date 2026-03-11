# Phase 35: Observability Foundation — UAT

**Date:** 2026-03-11
**Status:** PASSED (9/9)

## Tests

| # | Test | Status | Notes |
|---|------|--------|-------|
| 1 | gate_run command-only: warnings absent on success | PASS | skip_serializing_if omits empty vec |
| 2 | gate_run/finalize: warnings present on save failure | PASS | Read-only dir triggers save failure, warning surfaces |
| 3 | gate_finalize: persisted=true, correct struct on success | PASS | All fields present: run_id, spec_name, passed, failed, skipped, required_failed, advisory_failed, blocked, persisted |
| 4 | gate_finalize: persisted=false, warnings on save failure | PASS | Covered by test 2 (same test validates both) |
| 5 | gate_history: outcome=passed returns only passed runs | PASS | required_failed == 0 for all returned runs |
| 6 | gate_history: outcome=failed returns only failed runs | PASS | required_failed > 0 for all returned runs |
| 7 | gate_history: default limit=10 | PASS | 15 runs created, 10 returned without limit param |
| 8 | gate_history: limit capped at 50 | PASS | 51 runs created, limit=100 requested, 50 returned |
| 9 | gate_history: unrecognized outcome returns error | PASS | "garbage" outcome returns domain error |
