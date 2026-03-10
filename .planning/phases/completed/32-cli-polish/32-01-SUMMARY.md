# Phase 32 Plan 01: CLI Shared Constants & TTY Detection Summary

TTY-aware color detection, DIRECTORY_SPEC_INDICATOR constant in assay-types, and COLUMN_GAP constant for tabular formatting.

## Tasks Completed

| # | Task | Commit |
|---|------|--------|
| 1 | Add TTY detection to colors_enabled() (CLI-01) | fbc41de |
| 2 | Extract [srs] magic string to constant (CLI-08) | f1b2dda |
| 3 | Add COLUMN_GAP constant (CLI-07) | 6d0f5f2 |

## Key Changes

- **colors_enabled()** now returns false when stdout is not a TTY (piped output), in addition to the existing NO_COLOR check. Uses `std::io::IsTerminal`. No call-site changes needed; the `--plain` override pattern in context.rs is preserved.
- **DIRECTORY_SPEC_INDICATOR** (`"[srs]"`) defined in `assay-types/src/lib.rs` with doc comment (required by `#![deny(missing_docs)]`). Replaced 3 string literals: init.rs (1), spec.rs (2).
- **COLUMN_GAP** (`"  "`) defined in `commands/mod.rs`. Used as format argument (`gap = COLUMN_GAP`) in spec list, spec show criteria table, and init status output. Format strings with padding specifiers use `{gap}` interpolation rather than literal double-spaces.

## Deviations

None - plan executed exactly as written.

## Duration

~4 minutes
