# DiffTruncation uses usize for byte fields instead of u64

**Source:** PR review (Phase 44)
**Severity:** Suggestion
**File:** crates/assay-types/src/gate_run.rs

`DiffTruncation.original_bytes` and `truncated_bytes` use `usize` but the codebase convention for byte-adjacent fields is `u64` (e.g., `GuardConfig.soft_threshold_bytes`). `u64` would be more explicit about range and consistent with the existing pattern.
