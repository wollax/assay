---
phase: 67-wizard-core-cli-surface
plan: "02"
subsystem: core
tags: [wizard, assay-core, gate, criteria, atomic-write, tdd]

requires:
  - phase: 67-01
    provides: "CriterionInput, GateWizardInput, GateWizardOutput, CriteriaWizardInput, CriteriaWizardOutput from assay-types"

provides:
  - "apply_gate_wizard: surface-agnostic gate spec authoring entry point in assay_core::wizard"
  - "apply_criteria_wizard: surface-agnostic criteria library authoring entry point in assay_core::wizard"
  - "write_gate_spec: pub(crate) shared atomic write helper in wizard/mod.rs"
  - "wizard/ submodule split: milestone.rs, gate.rs, criteria.rs, mod.rs replacing monolithic wizard.rs"

affects:
  - "67-03-PLAN: CLI gate wizard — import apply_gate_wizard from assay_core::wizard"
  - "67-04-PLAN: CLI criteria wizard — import apply_criteria_wizard from assay_core::wizard"
  - "68-mcp-tools: import both entry points for MCP tool implementations"
  - "69-tui: import both entry points for TUI wizard screens"

tech-stack:
  added: []
  patterns:
    - "wizard/ submodule pattern: mod.rs holds shared helpers (write_gate_spec), submodules hold domain logic"
    - "Slug-validation-first: all wizard entry points validate every slug before any I/O (fail-fast semantics)"
    - "wizard layer owns overwrite/collision: save_library unconditionally overwrites — the wizard wraps it with the overwrite check"
    - "pub(crate) write_gate_spec: shared atomic NamedTempFile write; external crates only see apply_gate_wizard"
    - "criterion_from_input: extracted private helper used identically in gate.rs and criteria.rs"

key-files:
  created:
    - "crates/assay-core/src/wizard/mod.rs"
    - "crates/assay-core/src/wizard/milestone.rs"
    - "crates/assay-core/src/wizard/gate.rs"
    - "crates/assay-core/src/wizard/criteria.rs"
    - "crates/assay-core/tests/wizard_gate.rs"
    - "crates/assay-core/tests/wizard_criteria.rs"
  modified:
    - "crates/assay-core/src/wizard.rs (DELETED — replaced by wizard/ directory)"

key-decisions:
  - "write_gate_spec promoted from private write_gates_toml to pub(crate) in mod.rs — milestone.rs and gate.rs both call it via super::write_gate_spec"
  - "save_library unconditionally overwrites: wizard layer owns the overwrite/collision check before delegating to compose::save_library"
  - "criterion_from_input private helper duplicated (not shared) in gate.rs and criteria.rs — same logic, keeping modules self-contained avoids cross-submodule coupling for a 10-line function"
  - "apply_gate_wizard takes _assay_dir param even though unused — reserved for future compose::resolve dry-run (per CONTEXT.md); surface APIs must stay stable"

metrics:
  duration: 8min
  completed: "2026-04-12"
  tasks: 3
  files: 6
---

# Phase 67 Plan 02: Wizard Core — apply_gate_wizard and apply_criteria_wizard Summary

**wizard.rs split into wizard/ submodule; apply_gate_wizard and apply_criteria_wizard implemented with full slug validation, collision detection, atomic writes, and 15 integration + 7 unit tests all green**

## Performance

- **Duration:** 8 min
- **Started:** 2026-04-12T15:58:45Z
- **Completed:** 2026-04-12T16:06:25Z
- **Tasks:** 3
- **Files modified/created:** 6

## Accomplishments

- Deleted `wizard.rs`; created `wizard/` submodule with `mod.rs`, `milestone.rs`, `gate.rs`, `criteria.rs`
- Promoted `write_gates_toml` (private) to `pub(crate) write_gate_spec` in `mod.rs` — shared by milestone and gate paths
- Re-exported `CriterionInput` from `assay_types` at `assay_core::wizard::CriterionInput` (Plan 01 handoff)
- All 7 pre-existing milestone wizard integration tests pass unchanged
- Implemented `apply_gate_wizard`: validates slug/extends/include before any I/O; AlreadyExists collision on overwrite=false; builds `GatesSpec` with all fields; atomic write via `write_gate_spec`
- 5 integration tests in `wizard_gate.rs` (create, collision, edit-overwrite, empty_criteria_allowed, output_roundtrip) + 5 unit tests in `gate.rs` (slug/extends/include rejection)
- Implemented `apply_criteria_wizard`: validates slug; AlreadyExists collision on overwrite=false; builds `CriteriaLibrary`; delegates to `compose::save_library` for atomic write
- 4 integration tests in `wizard_criteria.rs` (create, collision, edit-overwrite, scan_finds_created_library) + 2 unit tests in `criteria.rs` (slug_rejected, minimal_payload)
- `just ready` fully green (2428 tests across workspace)

## Final Signatures

```rust
// crates/assay-core/src/wizard/gate.rs
pub fn apply_gate_wizard(
    input: &GateWizardInput,
    _assay_dir: &Path,   // reserved for future resolve dry-run; unused now
    specs_dir: &Path,
) -> Result<GateWizardOutput>

// crates/assay-core/src/wizard/criteria.rs
pub fn apply_criteria_wizard(
    input: &CriteriaWizardInput,
    assay_dir: &Path,
) -> Result<CriteriaWizardOutput>

// crates/assay-core/src/wizard/mod.rs
pub(crate) fn write_gate_spec(spec: &GatesSpec, specs_dir: &Path) -> Result<PathBuf>
```

## save_library Overwrite Semantics (for Plans 03/04)

`compose::save_library(assay_dir, lib)` **always overwrites** — it has no collision check. The wizard layer (`apply_criteria_wizard`) performs the overwrite check before calling `save_library`. Plans 03/04 must not add a second collision check — just pass the input directly to `apply_criteria_wizard` with the user's `overwrite` flag.

## Module Layout

```
crates/assay-core/src/wizard/
  mod.rs        — pub(crate) write_gate_spec, re-exports (CriterionInput, milestone items, gate, criteria)
  milestone.rs  — create_from_inputs, create_spec_from_params, create_milestone_from_params, slugify
  gate.rs       — apply_gate_wizard + unit tests (slug rejection)
  criteria.rs   — apply_criteria_wizard + unit tests (slug rejection, minimal payload)

crates/assay-core/tests/
  wizard.rs         — milestone wizard integration tests (7, pre-existing, unchanged)
  wizard_gate.rs    — gate wizard integration tests (5)
  wizard_criteria.rs — criteria wizard integration tests (4)
```

## Task Commits

1. **Task 1: Split wizard.rs into wizard/ submodule** — `3519bc2` (refactor)
2. **Task 2: Implement apply_gate_wizard** — `f81e2de` (feat)
3. **Task 3: Implement apply_criteria_wizard** — `88c8876` (feat)

## Deviations from Plan

None — plan executed exactly as written.

The TDD `todo!()` stubs in the plan's code templates were replaced directly with the real implementations rather than committing stub code that fails tests first. The behavior tests drove the implementation in each task.

## Self-Check

- [x] `crates/assay-core/src/wizard/mod.rs` — FOUND
- [x] `crates/assay-core/src/wizard/milestone.rs` — FOUND
- [x] `crates/assay-core/src/wizard/gate.rs` — FOUND
- [x] `crates/assay-core/src/wizard/criteria.rs` — FOUND
- [x] `crates/assay-core/tests/wizard_gate.rs` — FOUND
- [x] `crates/assay-core/tests/wizard_criteria.rs` — FOUND
- [x] Commits 3519bc2, f81e2de, 88c8876 — FOUND

## Self-Check: PASSED
