# Phase 11 Verification Report

**Phase:** 11 — Type System Foundation
**Verified:** 2026-03-04
**Method:** Goal-backward verification against actual codebase state

---

## Overall Status: PASS (with one caveat noted)

`just ready` passes: fmt-check + lint + test (171 passed, 3 ignored) + deny all pass.

---

## Truth Verification

### T1: GateRunSummary and CriterionResult are defined in assay-types, not assay-core

**PASS.**

`crates/assay-types/src/gate_run.rs` exists and defines both structs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GateRunSummary { ... }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CriterionResult { ... }
```

No `struct GateRunSummary` or `struct CriterionResult` definitions exist anywhere in `crates/assay-core/src/`.

---

### T2: No re-exports of GateRunSummary or CriterionResult from assay-core

**PASS.**

`crates/assay-core/src/lib.rs` exposes only module paths (`pub mod gate`, `pub mod spec`, etc.) and `pub use error::{AssayError, Result}`. No `pub use` for `GateRunSummary` or `CriterionResult`.

`crates/assay-core/src/gate/mod.rs` imports them from `assay_types` for internal use but does not re-export them.

---

### T3: All Option fields have skip_serializing_if + default paired together

**PASS.**

In `crates/assay-types/src/lib.rs`:
- `Config.gates: Option<GatesConfig>` → `#[serde(default, skip_serializing_if = "Option::is_none")]`
- `GatesConfig.working_dir: Option<String>` → `#[serde(default, skip_serializing_if = "Option::is_none")]`

In `crates/assay-types/src/gate_run.rs`:
- `CriterionResult.result: Option<GateResult>` → `#[serde(default, skip_serializing_if = "Option::is_none")]`

---

### T4: All Vec fields on Review and Workflow have skip_serializing_if + default

**PASS.**

In `crates/assay-types/src/lib.rs`:
- `Review.comments: Vec<String>` → `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- `Workflow.specs: Vec<Spec>` → `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- `Workflow.gates: Vec<Gate>` → `#[serde(default, skip_serializing_if = "Vec::is_empty")]`

---

### T5: Uncommitted files (feature_spec.rs, gates_spec.rs, modified files) are integrated

**PASS.**

`crates/assay-types/src/lib.rs` declares:
```rust
pub mod feature_spec;
pub mod gates_spec;
pub mod gate_run;
```

And re-exports:
```rust
pub use feature_spec::FeatureSpec;
pub use gate_run::{CriterionResult, GateRunSummary};
pub use gates_spec::{GateCriterion, GatesSpec};
```

All three new source files are wired into the crate root.

---

### T6: JSON Schema snapshots exist for GateRunSummary and CriterionResult

**PASS.**

Both snapshot files exist:
- `crates/assay-types/tests/snapshots/schema_snapshots__gate-run-summary-schema.snap`
- `crates/assay-types/tests/snapshots/schema_snapshots__criterion-result-schema.snap`

Both contain valid snapshot content (confirmed by reading file headers and content).

---

### T7: Roundtrip serde tests validate GateRunSummary and CriterionResult

**PASS.**

`crates/assay-types/tests/schema_roundtrip.rs` contains:
- `gate_run_summary_full_validates()` — tests a full GateRunSummary with results
- `gate_run_summary_with_skipped_criterion_validates()` — tests skipped criterion (None result)
- `criterion_result_with_result_validates()` — tests CriterionResult with result
- `criterion_result_skipped_validates()` — tests CriterionResult with None result
- `gate_run_summary_backward_compat_deserialize()` — tests deserialization of minimal JSON without the `results` field

---

### T8: All snapshot tests pass (no pending snapshots)

**PASS.**

`cargo test --workspace` reports 171 passed, 3 ignored, 0 failed. No pending snapshots.

---

### T9: schemas/ directory contains generated JSON schema files for new types

**PASS.**

Both required schema files exist in `schemas/`:
- `schemas/gate-run-summary.schema.json` (5.5K)
- `schemas/criterion-result.schema.json` (4.3K)

---

### T10: just ready passes

**PASS.**

`just ready` output: "All checks passed." — fmt-check, lint, test, and deny all succeed.

---

### T11: A v0.1.0-era spec file (without new fields) parses successfully

**PASS (satisfied via existing mechanisms).**

The `Spec.description` field uses `#[serde(default, skip_serializing_if = "String::is_empty")]`, so a v0.1.0-era TOML without `description` deserializes to `description: ""` without error.

The `load_spec_entry_finds_legacy` test in `crates/assay-core/src/spec/mod.rs` (line 1183) parses exactly this format:
```toml
name = "hello"

[[criteria]]
name = "c1"
description = "d1"
```

The `gate_run_summary_backward_compat_deserialize` roundtrip test covers GateRunSummary backward compat explicitly with a comment citing TYPE-03.

**Caveat:** There is no dedicated test named "v0.1.0-era spec file" that explicitly tests Spec deserialization from JSON/TOML without the `description` field in the roundtrip test file. The claim is satisfied indirectly: the `serde(default)` attr on `description` guarantees it, and the `load_spec_entry_finds_legacy` test exercises this parse path in assay-core tests. This is functionally correct but not a named test against the v0.1.0-era claim.

---

## Artifact Verification

### A1: crates/assay-types/src/gate_run.rs with GateRunSummary + CriterionResult

**PASS.** File exists at the required path with both structs deriving `Serialize, Deserialize, JsonSchema`. Also includes `inventory::submit!` entries for schema registry.

### A2: Snapshot files for gate-run-summary-schema and criterion-result-schema

**PASS.** Both exist in `crates/assay-types/tests/snapshots/`.

### A3: schemas/gate-run-summary.schema.json

**PASS.** Exists at `schemas/gate-run-summary.schema.json`.

### A4: schemas/criterion-result.schema.json

**PASS.** Exists at `schemas/criterion-result.schema.json`.

---

## Key Links Verification

### L1: assay-core::gate::evaluate_all returns assay_types::GateRunSummary

**PASS.** `crates/assay-core/src/gate/mod.rs` line 28 imports `GateRunSummary` from `assay_types` and uses it as return type for `evaluate_all` (line 89) and `evaluate_all_async` (line 163).

### L2: assay-mcp::server imports GateRunSummary/CriterionResult from assay_types

**PASS.** `crates/assay-mcp/src/server.rs` line 416: `use assay_types::{CriterionResult, GateKind, GateResult, GateRunSummary};` in tests module, and line 325 uses `assay_types::GateRunSummary` in the mapping function.

### L3: CriterionResult.result references assay_types::GateResult (no circular deps)

**PASS.** `gate_run.rs` uses `use crate::GateResult;` — `GateResult` is defined in `assay-types/src/gate.rs`. No circular dependency. `gate_run.rs` only imports from within `assay-types`.

### L4: inventory::submit! entries in gate_run.rs are discovered by schema_registry

**PASS.** `gate_run.rs` has two `inventory::submit!` blocks registering `gate-run-summary` and `criterion-result` entries into `crate::schema_registry::SchemaEntry`.

### L5: Schema snapshot tests use insta::assert_json_snapshot! pattern

**PASS.** `schema_snapshots.rs` uses `assert_json_snapshot!("gate-run-summary-schema", schema.to_value())` and `assert_json_snapshot!("criterion-result-schema", schema.to_value())`.

### L6: Roundtrip tests use the validate() helper from schema_roundtrip.rs

**PASS.** All roundtrip tests call the `validate()` helper which generates a schema and runs jsonschema Draft 2020-12 validation.

---

## ROADMAP Requirements

### TYPE-01: GateRunSummary and CriterionResult relocated with Deserialize + JsonSchema

**PASS.** Both structs are in `assay-types` with full `Serialize, Deserialize, JsonSchema` derives.

### TYPE-02: All domain types use #[serde(skip_serializing_if)] on optional fields

**PASS.** All Option and Vec fields on Review, Workflow, Config, GatesConfig, CriterionResult, GateRunSummary.results use the attribute.

### TYPE-03: New fields use #[serde(default)] for backward compatibility

**PASS.** Every `skip_serializing_if` is paired with `default` (or a custom default function). The backward compat test is named after TYPE-03.

---

## Success Criteria

1. **GateRunSummary and CriterionResult import from assay_types in all consuming crates and just ready passes** — PASS
2. **Serializing a type with None optional fields produces JSON without those keys** — PASS (skip_serializing_if on all optional fields; validated by roundtrip tests)
3. **A v0.1.0-era spec file (without new fields) parses successfully under v0.2.0 code** — PASS (serde(default) on description; load_spec_entry_finds_legacy test covers this)
4. **JSON Schema snapshots reflect the relocated types with Deserialize + JsonSchema derives** — PASS (snapshot files exist and tests pass)

---

## Verdict

**GOAL ACHIEVED.** All required truths hold, all artifacts exist, all key links are wired correctly, and `just ready` passes cleanly. The one noted caveat (no standalone named test specifically for "v0.1.0-era Spec TOML" in the roundtrip file) does not represent a goal failure — the backward compat claim is fully satisfied by `serde(default)` on `description` and the existing parse test in assay-core.
