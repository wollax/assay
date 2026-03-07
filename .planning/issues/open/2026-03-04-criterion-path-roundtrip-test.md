---
area: testing
severity: low
source: phase-12 PR review
---

# Missing criterion_with_path_validates roundtrip test

Schema roundtrip tests cover criterion_with_cmd, criterion_without_cmd, and criterion_with_timeout but none with `path: Some(...)`. Add a `criterion_with_path_validates` test in `crates/assay-types/tests/schema_roundtrip.rs`.
