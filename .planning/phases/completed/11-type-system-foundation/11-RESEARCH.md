# Phase 11: Type System Foundation — Research

## Standard Stack

**Confidence: HIGH** — All libraries already in workspace `Cargo.toml`.

| Purpose | Library | Version | Notes |
|---------|---------|---------|-------|
| Serialization | `serde` | 1 | Already derived on all domain types |
| JSON Schema | `schemars` | 1 | With `chrono04` feature enabled |
| Schema registry | `inventory` | 0.3 | Auto-discovery via `inventory::submit!` / `inventory::collect!` |
| Snapshot testing | `insta` | 1.46 | With `json` feature; use `cargo insta review` to accept |
| Schema validation | `jsonschema` | 0.43 | Draft 2020-12 validator in roundtrip tests |
| TOML serde | `toml` | 0.8 | Used for spec/config parsing |

No new dependencies needed. All workspace deps are already declared.

## Architecture Patterns

### Current Dependency Graph

```
assay-cli ──→ assay-core ──→ assay-types
    │              │
    └──→ assay-mcp ┘──→ assay-types (direct dep)
         └──→ assay-core
```

**Key finding:** `assay-mcp` already depends on BOTH `assay-core` AND `assay-types` directly (confirmed in `crates/assay-mcp/Cargo.toml`). Same for `assay-cli`. This means the "clean break" strategy is viable — consumers can import from `assay_types` directly without needing re-exports through `assay-core`.

### assay-types Module Structure (Current)

```
assay-types/src/
  lib.rs          → Spec, Gate, Review, Workflow, Config, GatesConfig + re-exports
  criterion.rs    → Criterion
  gate.rs         → GateKind, GateResult
  feature_spec.rs → FeatureSpec + ~15 supporting types (NEW, uncommitted)
  gates_spec.rs   → GatesSpec, GateCriterion (NEW, uncommitted)
  schema_registry.rs → SchemaEntry, all_entries()
```

### Types to Relocate from assay-core

| Type | Current Location | Derives (Current) | Derives (Needed) |
|------|-----------------|-------------------|-------------------|
| `GateRunSummary` | `assay-core::gate` | `Debug, Clone, Serialize` | Add `Deserialize, JsonSchema` |
| `CriterionResult` | `assay-core::gate` | `Debug, Clone, Serialize` | Add `Deserialize, JsonSchema` |

**Fields of GateRunSummary:**
- `spec_name: String`
- `results: Vec<CriterionResult>`
- `passed: usize`
- `failed: usize`
- `skipped: usize`
- `total_duration_ms: u64`

**Fields of CriterionResult:**
- `criterion_name: String`
- `result: Option<GateResult>` — references `GateResult` which is already in `assay-types`

Both types reference only types already in `assay-types` (`GateResult`, `GateKind`), so relocation is clean with no circular dependency risk.

### Other Misplaced Candidates Assessment

| Type | Location | Relocate? | Rationale |
|------|----------|-----------|-----------|
| `SpecError` | `assay-core::spec` | **No** | Validation logic type, not a DTO |
| `SpecEntry` | `assay-core::spec` | **No** | Contains runtime enum variants with loaded data, not serializable |
| `ScanResult` | `assay-core::spec` | **No** | Internal aggregation type for spec scanning |
| `ConfigError` | `assay-core::config` | **No** | Validation error type, not a DTO |
| `AssayError` | `assay-core::error` | **No** | Contains `std::io::Error` (not serializable), domain error enum |
| `InitOptions` / `InitResult` | `assay-core::init` | **No** | CLI-facing operation types, not domain DTOs |

**Conclusion:** Only `GateRunSummary` and `CriterionResult` need relocation. All other assay-core types are rightfully business logic types, not serializable DTOs.

### Recommended Module Placement in assay-types

Place `GateRunSummary` and `CriterionResult` in a new `gate_run.rs` module:
- `assay-types/src/gate_run.rs` — keeps gate evaluation result types separate from gate definition types (`gate.rs`)
- Re-export from `lib.rs`: `pub use gate_run::{GateRunSummary, CriterionResult};`

Rationale: `gate.rs` defines gate kinds and individual results. `gate_run.rs` defines aggregate evaluation results. This matches the existing pattern of `criterion.rs` vs `gates_spec.rs`.

### assay-mcp Dependency Routing Decision

**Recommendation: Keep assay-mcp importing from assay-types directly** (current state). The Cargo.toml already lists both `assay-core` and `assay-types` as deps. After relocation, `assay-mcp/src/server.rs` will change:

```rust
// Before:
use assay_core::gate::{CriterionResult, GateRunSummary};

// After:
use assay_types::{CriterionResult, GateRunSummary};
```

## Consumers That Need Import Updates

### Direct consumers of `GateRunSummary` / `CriterionResult`

| File | Current Import | Change To |
|------|---------------|-----------|
| `crates/assay-core/src/gate/mod.rs` | Defines locally | Remove definitions, add `use assay_types::{GateRunSummary, CriterionResult}` |
| `crates/assay-mcp/src/server.rs` | `use assay_core::gate::{CriterionResult, GateRunSummary}` (in tests) | `use assay_types::{CriterionResult, GateRunSummary}` |
| `crates/assay-cli/src/main.rs` | No direct import (uses `assay_core::gate::evaluate_all` return type) | No change needed — type inference handles it |

**Confidence: HIGH** — Grep confirms these are the only files referencing these types.

## Don't Hand-Roll

- **Schema registry entries** — Use the `inventory::submit!` macro pattern already established. Every type in `assay-types` registers itself at definition site.
- **Serde skip attributes** — Use `#[serde(skip_serializing_if = "Option::is_none")]` for `Option<T>` and `#[serde(skip_serializing_if = "Vec::is_empty")]` for `Vec<T>`. Do not write custom `is_empty` functions.
- **Schema snapshot tests** — Use `insta::assert_json_snapshot!` with `schema.to_value()` pattern already in `schema_snapshots.rs`.
- **Backward compatibility** — Use `#[serde(default)]` on new fields. Do NOT write custom deserializers.

## Serde Hygiene Audit

### Types Already Compliant (have proper skip_serializing_if)

These types already have correct serde hygiene on all `Option` and `Vec` fields:

- `GateResult` (gate.rs) — all optional fields handled
- `Criterion` (criterion.rs) — `cmd`, `timeout` handled
- `GateCriterion` (gates_spec.rs) — `cmd`, `timeout`, `requirements` handled
- `GatesSpec` (gates_spec.rs) — `description` handled
- `Spec` (lib.rs) — `description` handled
- `Config` (lib.rs) — `gates` handled
- `GatesConfig` (lib.rs) — `working_dir` handled
- `FeatureSpec` (feature_spec.rs) — all fields handled
- All sub-types in feature_spec.rs — all fields handled

### Types That Need Serde Hygiene

| Type | Field | Current | Needed |
|------|-------|---------|--------|
| `Gate` | `passed: bool` | No skip | N/A — bool is always present, no action needed |
| `Review` | `comments: Vec<String>` | No skip | Add `#[serde(default, skip_serializing_if = "Vec::is_empty")]` |
| `Workflow` | `specs: Vec<Spec>` | No skip | Add `#[serde(default, skip_serializing_if = "Vec::is_empty")]` |
| `Workflow` | `gates: Vec<Gate>` | No skip | Add `#[serde(default, skip_serializing_if = "Vec::is_empty")]` |

**Assessment:** `Gate`, `Review`, and `Workflow` are the oldest types — they predate the serde hygiene patterns established in later work. The `Gate` struct is structurally fine (no optional fields). `Review.comments` and `Workflow.{specs,gates}` are the only gaps.

### New Types Needing Hygiene (GateRunSummary, CriterionResult)

After relocation, add:
- `CriterionResult.result: Option<GateResult>` — needs `#[serde(skip_serializing_if = "Option::is_none", default)]`
- `GateRunSummary` — all fields are required scalars or `Vec<CriterionResult>`. The `results` vec should get `#[serde(default, skip_serializing_if = "Vec::is_empty")]` (though in practice it's never empty).

## Common Pitfalls

### 1. Forgetting `default` alongside `skip_serializing_if` (Confidence: HIGH)
If you add `skip_serializing_if` without `default`, deserialization will fail when the field is absent in input. Always pair them:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub field: Option<T>,
```

### 2. Breaking the CLI JSON output (Confidence: HIGH)
`assay gate run --json` serializes `GateRunSummary` directly via `serde_json::to_string_pretty`. After relocation, the type must produce identical JSON output. The `Serialize` derive will be the same, but verify by running `just test` which exercises the MCP server's `format_gate_response` tests.

### 3. Schema snapshot drift (Confidence: HIGH)
Adding `Deserialize` and `JsonSchema` to `GateRunSummary`/`CriterionResult` will create new schema entries. New snapshot tests must be added, and `cargo insta review` must be run to accept them. Forgetting to add `inventory::submit!` will cause the schema to be missing from the registry.

### 4. Serde attribute ordering on relocated types (Confidence: MEDIUM)
The existing types use two attribute styles:
- `#[serde(skip_serializing_if = "Option::is_none", default)]` (Option::is_none first)
- `#[serde(default, skip_serializing_if = "...")]` (default first)

Both work identically. For consistency with newer types (feature_spec.rs, gates_spec.rs), use `default` first.

### 5. `deny_unknown_fields` consideration (Confidence: MEDIUM)
Most types in assay-types use `#[serde(deny_unknown_fields)]` (Spec, Config, GatesConfig, FeatureSpec, GatesSpec, etc.). `GateRunSummary` and `CriterionResult` are output types (produced by gate evaluation, consumed by CLI/MCP). Adding `deny_unknown_fields` would make them less forward-compatible. **Recommendation: Do NOT add `deny_unknown_fields`** to these types — they are results, not user-authored configs.

### 6. Existing tests constructing GateRunSummary directly (Confidence: HIGH)
Several test files construct `GateRunSummary` and `CriterionResult` directly:
- `crates/assay-mcp/src/server.rs` tests (lines 421-467) — `sample_summary()` helper
- `crates/assay-core/src/gate/mod.rs` tests — via `evaluate_all()` return values

After relocation, the MCP server tests must update their imports. The core gate tests don't import the types directly (they use return values), so they should be fine.

### 7. Uncommitted work integration (Confidence: HIGH)
The working tree has uncommitted changes across multiple files:
- `feature_spec.rs` and `gates_spec.rs` (new files in assay-types)
- Modified: `error.rs`, `gate/mod.rs`, `init.rs`, `spec/mod.rs`, `server.rs`, `lib.rs`
- New schema files and snapshots

These are all coherent additions from previous work. Phase 11 must integrate them as-is and commit everything together. The new files already follow all serde conventions.

## Code Examples

### Relocated GateRunSummary (target state)

```rust
// crates/assay-types/src/gate_run.rs

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::GateResult;

/// Summary of evaluating all criteria in a spec.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GateRunSummary {
    /// Spec name that was evaluated.
    pub spec_name: String,
    /// Results for each criterion.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<CriterionResult>,
    /// Number of criteria that passed.
    pub passed: usize,
    /// Number of criteria that failed.
    pub failed: usize,
    /// Number of criteria skipped.
    pub skipped: usize,
    /// Total wall-clock duration in milliseconds.
    pub total_duration_ms: u64,
}

/// A criterion paired with its evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CriterionResult {
    /// The name of the criterion.
    pub criterion_name: String,
    /// The gate result, or `None` if skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<GateResult>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-run-summary",
        generate: || schemars::schema_for!(GateRunSummary),
    }
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "criterion-result",
        generate: || schemars::schema_for!(CriterionResult),
    }
}
```

### Updated assay-core gate/mod.rs import

```rust
// Remove local struct definitions, add:
use assay_types::{CriterionResult, GateRunSummary};
```

### Schema snapshot test additions

```rust
#[test]
fn gate_run_summary_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::GateRunSummary);
    assert_json_snapshot!("gate-run-summary-schema", schema.to_value());
}

#[test]
fn criterion_result_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::CriterionResult);
    assert_json_snapshot!("criterion-result-schema", schema.to_value());
}
```

### Roundtrip test additions

```rust
#[test]
fn gate_run_summary_validates() {
    validate(&GateRunSummary {
        spec_name: "test-spec".to_string(),
        results: vec![CriterionResult {
            criterion_name: "unit-tests".to_string(),
            result: Some(GateResult {
                passed: true,
                kind: GateKind::Command { cmd: "cargo test".to_string() },
                stdout: String::new(),
                stderr: String::new(),
                exit_code: Some(0),
                duration_ms: 100,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
            }),
        }],
        passed: 1,
        failed: 0,
        skipped: 0,
        total_duration_ms: 100,
    });
}
```

### Serde hygiene fix for Review

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Review {
    pub spec_name: String,
    pub approved: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}
```

## Discretion Recommendations

### Module structure: `gate_run.rs` submodule
Use a separate `gate_run.rs` file rather than adding to `gate.rs`. The existing `gate.rs` defines gate _kinds_ and individual _results_. Aggregate run summaries are a different concern. This keeps each module focused.

### Whether to front-load Phase 13 `enforcement` field
**Recommendation: Do NOT add `enforcement` field now.** Phase 11 is about relocation and hygiene. Adding a field that won't be used until Phase 13 increases review surface without benefit. The `#[serde(default)]` pattern established here makes adding it later trivial and backward-compatible.

### Schema update pass strategy
**Recommendation: Inline with type changes.** Each type modification (adding derives, adding serde attrs) should update its snapshot in the same task. Running `cargo insta review --accept` once at the end is simpler than batching.

### Snapshot test naming
**Recommendation: Use type-name convention** (matching existing pattern: `"spec-schema"`, `"gate-result-schema"`, etc.). New entries: `"gate-run-summary-schema"`, `"criterion-result-schema"`.

### schemas/ directory regeneration
The `schemas/` directory contains pre-generated `.schema.json` files. New types (`gate-run-summary`, `criterion-result`) need entries. **Recommendation: Regenerate at the end of the phase** after all type work is complete, since the schema generator iterates `inventory` entries.

## Verification Checklist

After all changes:
1. `just ready` passes (fmt-check + lint + test + deny)
2. `cargo insta test` shows no pending snapshots
3. `GateRunSummary` and `CriterionResult` are importable from `assay_types`
4. No references to `assay_core::gate::GateRunSummary` or `assay_core::gate::CriterionResult` remain
5. Serializing a `Review` with empty `comments` omits the field from JSON
6. A minimal spec TOML (v0.1.0 format, without new fields) still parses
7. `schemas/` directory contains `gate-run-summary.schema.json` and `criterion-result.schema.json`
