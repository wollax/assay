# Phase 44: gate_evaluate Context Budgeting — UAT

**Phase:** 44
**Started:** 2026-03-15
**Status:** PASSED (10/10)

## Tests

| # | Test | Status | Notes |
|---|------|--------|-------|
| 1 | DiffTruncation struct has correct fields and derives | PASS | 4 fields, 7 derives, schema registration |
| 2 | GateRunRecord backward compat — old records without diff_truncation deserialize | PASS | serde(default) on Option field with deny_unknown_fields |
| 3 | context_window_for_model accessible from assay_core::context | PASS | pub use re-export in context/mod.rs, used in server.rs:1344 |
| 4 | extract_diff_files parses git diff headers correctly | PASS | 5 unit tests pass (empty, single, multi, no headers, spaces) |
| 5 | Schema snapshots include DiffTruncation and updated GateRunRecord | PASS | Both snapshot files exist and pass |
| 6 | gate_evaluate uses budget_context instead of truncate_diff | PASS | budget_context at line 1355; truncate_diff only in gate_run and fallback |
| 7 | Truncation metadata populated when diff exceeds budget | PASS | DiffTruncation built with file lists, warning emitted, record updated |
| 8 | No truncation metadata when diff fits within budget | PASS | Clean passthrough returns (diff, None) |
| 9 | budget_context failure falls back to byte truncation with warning | PASS | truncate_diff as safety net, descriptive warning |
| 10 | Full test suite passes | PASS | just ready: 794 tests, fmt + lint + deny clean |
