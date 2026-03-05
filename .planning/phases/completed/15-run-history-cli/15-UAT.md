# Phase 15: Run History CLI — UAT Results

**Date:** 2026-03-05
**Tester:** Manual (conversational)
**Result:** 6/6 PASSED

## Tests

### Test 1: Empty history display
**Expected:** Running `assay gate history` on a spec with no history shows "No history" message and exits 0.
**Result:** PASS — Output: `No history found for 'hello-world'`, exit code 0.

### Test 2: Unknown spec error
**Expected:** Running `assay gate history --name nonexistent` shows error about spec not found, exits non-zero.
**Result:** PASS — Output: `Error: spec 'nonexistent' not found in specs/`, exit code 1.

### Test 3: Gate run saves history and table display
**Expected:** After running a gate, `assay gate history` shows a table with at least 1 row including correct pass/fail/skip counts.
**Result:** PASS — Table displayed with 1 row, correct columns (Run ID, Spec, Result, Passed, Failed, Skipped, Duration, Enforcement, Timestamp), counts matched gate output.

### Test 4: JSON output mode
**Expected:** `assay gate history --json` outputs valid JSON array parseable by `jq`.
**Result:** PASS — Valid JSON array with 1 element, all expected fields present, successfully parsed by `jq`.

### Test 5: Last run detail view
**Expected:** `assay gate history --last` shows formatted detail view with all fields.
**Result:** PASS — Detail view displayed with Run ID, Spec, Result, Timestamp, Duration, Working Directory, Assay Version, and Enforcement sections.

### Test 6: Pruning with max_history
**Expected:** With `max_history: 2` configured, running 3 gates results in only 2 history files remaining, with prune messages shown.
**Result:** PASS — "Pruned 1 old run(s) for check" messages displayed, only 2 JSON files remaining in results directory.
