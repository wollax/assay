---
phase: 67-wizard-core-cli-surface
plan: "01"
subsystem: types
tags: [serde, schemars, wizard, assay-types, jsonschema]

requires:
  - phase: 65-resolution-core
    provides: "CriteriaLibrary, GatesSpec, SpecPreconditions types used as fields"
  - phase: 64-type-foundation
    provides: "CriteriaLibrary, SpecPreconditions — preconditions as Option<SpecPreconditions>"

provides:
  - "CriterionInput: re-homed from assay-core::wizard into assay-types, now derives JsonSchema"
  - "GateWizardInput: full gate wizard payload with slug, description, extends, include, criteria, preconditions, overwrite"
  - "GateWizardOutput: gate wizard result carrying path + GatesSpec"
  - "CriteriaWizardInput: criteria library wizard payload with name, description, version, tags, criteria, overwrite"
  - "CriteriaWizardOutput: criteria wizard result carrying path + CriteriaLibrary"

affects:
  - "67-02-PLAN: must update assay-core::wizard to re-export CriterionInput from assay-types"
  - "68-mcp-tools: gets free JsonSchema generation for wizard input types"

tech-stack:
  added: []
  patterns:
    - "Surface-agnostic wizard input types in assay-types (not assay-core) for JsonSchema + multi-surface reuse"
    - "deny_unknown_fields on input types, not output types (forward-compat asymmetry)"

key-files:
  created:
    - "crates/assay-types/src/wizard_input.rs"
  modified:
    - "crates/assay-types/src/lib.rs"

key-decisions:
  - "CriterionInput re-homed from assay-core::wizard into assay-types — Plan 02 must update assay-core::wizard with pub use assay_types::CriterionInput"
  - "deny_unknown_fields applied to all wizard INPUT types (GateWizardInput, CriteriaWizardInput, CriterionInput) for loud MCP caller failures; OUTPUT types (GateWizardOutput, CriteriaWizardOutput) do NOT use it for forward-compatibility"
  - "Output types derive Serialize + JsonSchema only (no Deserialize) — they are only serialized for display/MCP responses"

patterns-established:
  - "Wizard types pattern: surface-agnostic input structs in assay-types, apply_* functions in assay-core"

requirements-completed: [WIZC-01, WIZC-02, WIZC-03]

duration: 3min
completed: "2026-04-12"
---

# Phase 67 Plan 01: Wizard Input Types Summary

**Five surface-agnostic wizard input/output types (CriterionInput, GateWizardInput, GateWizardOutput, CriteriaWizardInput, CriteriaWizardOutput) added to assay-types with Serialize/Deserialize/JsonSchema derives and 13 roundtrip/schema tests**

## Performance

- **Duration:** 3 min
- **Started:** 2026-04-12T15:53:48Z
- **Completed:** 2026-04-12T15:57:00Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- Created `wizard_input.rs` with all 5 structs required by the plan — CriterionInput (re-homed), GateWizardInput, GateWizardOutput, CriteriaWizardInput, CriteriaWizardOutput
- Applied `#[serde(deny_unknown_fields)]` to all input types; output types left without it for forward-compat
- 13 unit tests (TOML + JSON roundtrips, deny_unknown_fields rejection, JsonSchema non-empty assertions) all green
- Re-exported all 5 types from `assay-types::lib.rs`

## Task Commits

Each task was committed atomically:

1. **Task 1: Re-home CriterionInput and add GateWizardInput/Output + CriteriaWizardInput** - `47eb29a` (feat)

**Plan metadata:** _(docs commit follows)_

## Files Created/Modified
- `crates/assay-types/src/wizard_input.rs` - 5 wizard input/output types with 13 unit tests
- `crates/assay-types/src/lib.rs` - `pub mod wizard_input` added; 5 types re-exported

## Decisions Made

1. **CriterionInput re-homed into assay-types:** The existing `CriterionInput` in `assay-core::wizard` did not derive `JsonSchema` or `Serialize`/`Deserialize`. Plan 02 must add `pub use assay_types::CriterionInput;` to `assay-core::wizard` so downstream callers keep compiling.

2. **deny_unknown_fields asymmetry:** Input types use it so MCP callers get loud failures on field typos; output types don't use it so future fields can be added without breaking deserialization on old consumers.

3. **Output types are Serialize-only:** `GateWizardOutput` and `CriteriaWizardOutput` are never deserialized by callers — they are only produced and displayed. `Deserialize` was intentionally omitted.

## Deviations from Plan

None - plan executed exactly as written.

Minor deviation: `cargo fmt` reordered the `pub use wizard_input` re-export after fmt-check caught it in the pre-commit hook. Corrected inline.

## Issues Encountered

Pre-existing clippy warning in `manifest.rs:245` (`struct update has no effect`) was flagged by `--all-targets` but is out of scope for this plan. Logged to `deferred-items.md` in the phase directory.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02 can directly import `assay_types::{CriterionInput, GateWizardInput, GateWizardOutput, CriteriaWizardInput, CriteriaWizardOutput}` without modification
- **Critical for Plan 02:** Update `assay-core::wizard` to add `pub use assay_types::CriterionInput;` — the current `CriterionInput` struct in `wizard.rs` must be replaced with this re-export so existing `create_spec_from_params` and `write_gates_toml` callers keep compiling

---
*Phase: 67-wizard-core-cli-surface*
*Completed: 2026-04-12*
