# test_prune_none_means_no_pruning is redundant

**Area:** core/history
**Severity:** suggestion
**Source:** Phase 15 PR review

## Description

Every pre-existing history test already passes `None` to `save()`. The dedicated `test_prune_none_means_no_pruning` test adds no new coverage beyond what existing tests implicitly verify. Consider removing it to reduce test maintenance.

**File:** `crates/assay-core/src/history/mod.rs`


## Resolution

Closed as acknowledged in Phase 19 Plan 02 (2026-03-06). The redundant test is harmless and provides additional documentation of the None-means-no-pruning behavior. Not worth removing.
