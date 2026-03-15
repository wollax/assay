# Empty/whitespace `depends` entries not validated

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

`validate_spec` does not reject empty strings or whitespace-only strings in the `depends` field. These entries pass through and generate confusing "unknown dependency" diagnostics rather than a clear "invalid dependency name" error, making it harder for users to understand what went wrong.

## Suggested Fix

Add a validation pass over `depends` entries before dependency resolution. Entries that are empty or contain only whitespace should produce a targeted `Error` diagnostic with a message like `"dependency name must not be empty or whitespace"`.
