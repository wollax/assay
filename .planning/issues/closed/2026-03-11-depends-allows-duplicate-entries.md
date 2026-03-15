# depends allows duplicate entries

**Source:** Phase 37 PR review (type-reviewer)
**Area:** assay-types
**File(s):** crates/assay-types/src/lib.rs

## Description

The `depends` field is typed as `Vec<String>`, which permits duplicate entries such as `depends = ["auth", "auth"]`. Duplicates are semantically meaningless for a dependency list and will likely cause confusing behavior during cycle detection or dependency resolution — either silently ignored or triggering a spurious cycle error. There is no validation or warning to guide users away from this mistake.

## Suggested Fix

Either deserialize into a `BTreeSet<String>` to deduplicate at parse time, or add a validation step in `validate_spec` that emits a warning diagnostic when duplicate dependency slugs are detected.
