# 13-03 Summary: CLI Enforcement Integration

**Phase:** 13-enforcement-levels
**Plan:** 03
**Status:** Complete
**Duration:** ~23 minutes
**Date:** 2026-03-04

## One-liner

Updated CLI exit codes to use enforcement-aware logic, added `[gate]` section to spec template, and regenerated all schema snapshots.

## Tasks Completed

### Task 1: Update CLI exit code logic and spec new template for enforcement
- Added `required_failed` field to `StreamCounters`
- Updated `stream_criterion()` to accept resolved enforcement and track required failures
- Updated `print_gate_summary()` to exit 1 only when `required_failed > 0`
- Updated JSON mode exit codes in `handle_gate_run()` and `handle_gate_run_all()` to use `enforcement.required_failed`
- Updated `handle_spec_new()` template to include `[gate]` section with `enforcement = "required"`
- **Commit:** `f93d3be`

### Task 2: Regenerate schema snapshots, add roundtrip tests, run just ready
- Added 3 new schema snapshot tests: Enforcement, GateSection, EnforcementSummary
- Added 6 roundtrip serde tests: enforcement_roundtrip, gate_section_roundtrip, enforcement_summary_roundtrip, spec_with_gate_section_toml_roundtrip, spec_without_enforcement_fields_backward_compat, gate_run_summary_backward_compat_no_enforcement
- Regenerated all JSON schema files in `schemas/` directory (3 new, 7 updated)
- All existing snapshots updated to reflect enforcement fields
- `just ready` passes with zero warnings and zero failures
- **Commit:** `27b5ec4`

## Deviations

1. **clippy::too_many_arguments** -- Adding the `enforcement` parameter to `stream_criterion()` pushed it to 8 arguments (clippy limit is 7). Added `#[allow(clippy::too_many_arguments)]` attribute. A future refactor could bundle context parameters into a struct.

## Decisions

- Used `#[allow(clippy::too_many_arguments)]` on `stream_criterion()` rather than introducing a context struct, keeping the change minimal and focused on enforcement wiring.

## Artifacts

- `crates/assay-cli/src/main.rs` -- enforcement-aware exit codes and updated spec template
- `crates/assay-types/tests/schema_snapshots.rs` -- 3 new snapshot tests
- `crates/assay-types/tests/schema_roundtrip.rs` -- 6 new roundtrip tests
- `crates/assay-types/tests/snapshots/schema_snapshots__enforcement-schema.snap` -- new
- `crates/assay-types/tests/snapshots/schema_snapshots__enforcement-summary-schema.snap` -- new
- `crates/assay-types/tests/snapshots/schema_snapshots__gate-section-schema.snap` -- new
- `schemas/enforcement.schema.json` -- new
- `schemas/enforcement-summary.schema.json` -- new
- `schemas/gate-section.schema.json` -- new
- 7 existing schema files updated with enforcement fields

## Verification

- `just ready` passes (fmt-check + lint + test + deny)
- `cargo insta test -p assay-types` shows zero pending snapshots
- 188 tests pass, 3 ignored, zero failures
- All 3 exit code paths use `required_failed > 0`
- Phase 13 complete
