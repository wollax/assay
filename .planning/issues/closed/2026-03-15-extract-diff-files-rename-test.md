# extract_diff_files not tested for rename diffs

**Source:** PR review (Phase 44)
**Severity:** Suggestion
**File:** crates/assay-core/src/gate/mod.rs

Git rename diffs produce headers like `diff --git a/old/path.rs b/new/path.rs`. The function correctly returns the `b/` path, but a test covering rename diffs would make the intent explicit and prevent regressions.
