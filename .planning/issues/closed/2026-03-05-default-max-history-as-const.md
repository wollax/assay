# Default max_history value should be an associated constant

**Area:** types
**Severity:** suggestion
**Source:** Phase 15 PR review

## Description

If a default max_history value is reintroduced (removed as dead code in Phase 15 review fixes), it should be an associated constant on `GatesConfig` (e.g., `GatesConfig::DEFAULT_MAX_HISTORY`) rather than a free function, for better discoverability and association.

**File:** `crates/assay-types/src/lib.rs`
