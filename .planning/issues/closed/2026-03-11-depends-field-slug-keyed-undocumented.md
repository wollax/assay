# depends field not documented as slug-keyed

**Source:** Phase 37 PR review (type-reviewer)
**Area:** assay-types
**File(s):** crates/assay-types/src/lib.rs, crates/assay-types/src/gates_spec.rs

## Description

The `depends` field accepts a list of gate identifiers but its doc comment does not state that these must be slugs rather than display names. A user reading only the doc comment could reasonably write a display name (e.g., `"Auth Flow"`) expecting it to work, and would receive a confusing runtime error instead of a clear upfront contract. This is especially likely because other TOML fields in the same struct do use human-readable strings.

## Suggested Fix

Update the doc comment for `depends` to explicitly state that each entry must be the slug (kebab-case identifier) of another gate, and ideally link to wherever the slug format is defined or documented.
