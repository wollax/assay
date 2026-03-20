---
id: T02
parent: S03
milestone: M005
provides:
  - "assay_core::wizard module with WizardChunkInput, WizardInputs, WizardResult, CriterionInput structs"
  - "slugify() pure function"
  - "create_from_inputs() — atomic milestone + per-chunk gates.toml creation"
  - "create_milestone_from_params() — MCP-facing milestone creation"
  - "create_spec_from_params() — MCP-facing spec creation with milestone patching"
key_files:
  - crates/assay-core/src/wizard.rs
  - crates/assay-core/src/lib.rs
key_decisions:
  - "WizardChunkInput has slug/name/criteria: Vec<String> (not Vec<CriterionInput>) — tests are the authoritative contract; T02 plan's CriterionInput shape differed but the test signatures override it"
  - "WizardInputs has a slug field directly (not derived from name via slugify) — tests pass slug explicitly"
  - "create_spec_from_params signature is (slug, name, milestone_slug, assay_dir, specs_dir) — no order/criteria params, matching T01 test contract"
  - "CriterionInput is defined as pub struct with name/description/cmd fields for MCP use (T04), but wizard core tests use Vec<String>"
  - "write_gates_toml helper is private; the public API is create_from_inputs and create_spec_from_params"
patterns_established:
  - "Atomic gates.toml write via NamedTempFile::new_in + write_all + sync_all + persist — same pattern as milestone_save"
  - "create_dir_all before NamedTempFile::new_in to ensure the directory exists before placing temp file in it"
  - "Slug collision check: if milestone_file.exists() → AssayError::Io with AlreadyExists kind"
  - "Spec dir collision check: if specs_dir.join(slug).exists() → AssayError::Io"
  - "Milestone patching in create_spec_from_params: reload → push ChunkRef → save (idempotent pattern)"
observability_surfaces:
  - "WizardResult { milestone_path, spec_paths: Vec<PathBuf> } — callers can print all created paths"
  - "AssayError::Io with operation label 'milestone <slug> already exists' on slug collision"
  - "AssayError::Io with operation label 'spec directory <slug> already exists' on duplicate spec"
  - "milestone_load error propagates for non-existent milestone_slug in create_spec_from_params"
duration: short
verification_result: passed
completed_at: 2026-03-20
blocker_discovered: false
---

# T02: Implement `assay-core::wizard` Module

**Implemented the `assay_core::wizard` module with 5 public types and 4 public functions; all 5 wizard integration tests pass with zero regressions across the 680-test core suite.**

## What Happened

Created `crates/assay-core/src/wizard.rs` with the full wizard module:

**Types:**
- `CriterionInput { name, description, cmd }` — for MCP layer (T04)
- `WizardChunkInput { slug, name, criteria: Vec<String> }` — test-authoritative shape
- `WizardInputs { slug, name, description, chunks }` — slug is provided directly (not derived)
- `WizardResult { milestone_path, spec_paths }` — structured result for callers

**Functions:**
- `slugify(s)` — lowercase + collapse non-alphanumeric to `-` + trim hyphens; panics on empty result
- `create_from_inputs(inputs, assay_dir, specs_dir)` — validates slug, rejects collision, builds `Milestone` with `ChunkRef` entries, calls `milestone_save`, writes one `gates.toml` per chunk atomically
- `create_milestone_from_params(slug, name, desc, chunks, assay_dir)` — MCP-facing; validates + collision-checks + saves milestone
- `create_spec_from_params(slug, name, milestone_slug, assay_dir, specs_dir)` — validates, checks spec dir doesn't exist, loads milestone (fail if not found), writes gates.toml, patches milestone.chunks

**Private helper:** `write_gates_toml` encapsulates the `NamedTempFile` atomic write pattern for reuse by both `create_from_inputs` and `create_spec_from_params`.

Added `pub mod wizard;` to `crates/assay-core/src/lib.rs`.

## Verification

```
cargo test -p assay-core --features assay-types/orchestrate --test wizard
# 5 passed, 0 failed

cargo test -p assay-core --features assay-types/orchestrate
# 680 passed, 0 failed (no regression)
```

## Diagnostics

- Run `assay milestone list` in a project where `create_from_inputs` was called to see the generated milestone
- Run `assay gate run <chunk-slug>` to validate a generated `gates.toml` parses and runs cleanly
- `AssayError::Io` messages include slug/path context for all failure modes

## Deviations

- **Test contract overrides plan**: T01 tests use `WizardChunkInput { slug, name, criteria: Vec<String> }` (slug provided directly), while the T02 plan specified `ChunkInput { name, criteria: Vec<CriterionInput> }` with slug auto-derived via `slugify`. The tests are the authoritative contract; the implementation follows the tests. `CriterionInput` is still defined as a pub struct for the MCP layer (T04) but is not used in the wizard core tests.
- **`create_spec_from_params` signature**: Tests pass `(slug, name, milestone_slug, assay_dir, specs_dir)` — no `order` or `criteria` parameters as the T02 plan listed. Implementation matches the tests.

## Known Issues

None. MCP tool tests (`milestone_create`, `spec_create`) remain failing — they require T04's `MilestoneCreateParams`, `SpecCreateParams`, and server methods, which are not yet implemented.

## Files Created/Modified

- `crates/assay-core/src/wizard.rs` — complete wizard module (5 types, 4 public functions, 1 private helper)
- `crates/assay-core/src/lib.rs` — added `pub mod wizard;`
