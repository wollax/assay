# History table "pass/fail" status uses failed==0 instead of enforcement semantics

**Area:** cli
**Severity:** suggestion
**Source:** Phase 15 PR review

## Description

The history table view marks a run as "pass" when `s.failed == 0`, but the exit code logic uses `enforcement.required_failed > 0`. A run with only advisory failures shows as "fail" in the table but would exit 0. Consider aligning the status display with enforcement semantics (pass = no required failures).

**File:** `crates/assay-cli/src/main.rs`
