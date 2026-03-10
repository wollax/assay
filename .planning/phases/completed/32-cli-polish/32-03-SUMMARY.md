# Phase 32 Plan 03: Help Text Deduplication & Color Branch Collapse Summary

Removed duplicated after_long_help from top-level Spec/Gate commands and collapsed color-branching duplication in spec show table output.

## Tasks Completed

| # | Task | Commit |
|---|------|--------|
| 1 | Remove duplicated help text (CLI-02) | bf3b0ed |
| 2 | Collapse color branch duplication (CLI-04) | 70d62ea |

## Key Changes

- **main.rs**: Removed `after_long_help` attributes from `Spec` and `Gate` variants in the `Command` enum. Examples now live only at the subcommand level (`GateCommand::Run`, `SpecCommand::Show`, etc.). Top-level `assay spec --help` and `assay gate --help` show clap's default subcommand listing.
- **spec.rs**: Collapsed two identical `println!` branches (color vs plain) in `print_criteria_table` into a single call with a computed `tw` variable that conditionally adds `ANSI_COLOR_OVERHEAD`.

## Deviations

None - plan executed exactly as written. gate.rs color branch deduplication deferred to Plan 04 as specified.

## Duration

~3 minutes
