# Phase 13: Enforcement Levels - Research

**Researched:** 2026-03-04
**Confidence:** HIGH (all patterns are internal codebase extrapolation, no external libraries)

---

## Standard Stack

No new dependencies. This phase uses only existing crate features:

- `serde` (Serialize/Deserialize derive + `rename_all`, `default`, `deny_unknown_fields`)
- `schemars` (JsonSchema derive for automatic schema generation)
- `toml` (TOML serialization/deserialization)
- `inventory` (schema registry auto-discovery)

Everything lives in `assay-types` (type definitions) and `assay-core` (evaluation logic + validation).

---

## Architecture Patterns

### Pattern 1: Enum with kebab-case serde (for `Enforcement`)

The codebase uses `#[serde(rename_all = "kebab-case")]` on enums consistently. See `feature_spec.rs`:

```rust
// From crates/assay-types/src/feature_spec.rs:12-22
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SpecStatus {
    #[default]
    Draft,
    Proposed,
    // ...
}
```

**For `Enforcement`:** Use the same pattern. However, since the values are single lowercase words (`required`, `advisory`), `rename_all = "lowercase"` is more precise. But `kebab-case` produces identical output for single-word variants and is the established codebase convention. **Use `rename_all = "kebab-case"`.**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Enforcement {
    #[default]
    Required,
    Advisory,
}
```

**Note:** Add `Copy` because this is a two-variant fieldless enum. Every other similar enum in the codebase (`SpecStatus`, `Obligation`, `Priority`) derives `Clone` but not `Copy` -- however, `Enforcement` will be read frequently during evaluation and copied into results, so `Copy` is practical. Follow existing convention (no `Copy`) if consistency is preferred.

### Pattern 2: Output types use `serde(default)` for backward compat

From STATE.md: "Output types (GateRunSummary, CriterionResult) do NOT use deny_unknown_fields" and "New fields use serde(default) for backward compat."

Current `GateRunSummary` (from `crates/assay-types/src/gate_run.rs`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GateRunSummary {
    pub spec_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<CriterionResult>,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_duration_ms: u64,
}
```

Note: No `#[serde(deny_unknown_fields)]` on `GateRunSummary` or `CriterionResult`. New fields can be added with `#[serde(default)]`.

Current `CriterionResult`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CriterionResult {
    pub criterion_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<GateResult>,
}
```

### Pattern 3: Input types use `deny_unknown_fields`

`Criterion` and `GateCriterion` both use `#[serde(deny_unknown_fields)]`:

```rust
// crates/assay-types/src/criterion.rs:12-13
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Criterion { ... }
```

Adding `enforcement` field to these structs is safe because `deny_unknown_fields` only rejects fields NOT in the struct. Adding a new field with `#[serde(default)]` means old TOML files (without the field) still parse fine.

### Pattern 4: Schema registry via `inventory::submit!`

Every type in `assay-types` registers itself:

```rust
inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "enforcement",
        generate: || schemars::schema_for!(Enforcement),
    }
}
```

New types (`Enforcement`, `EnforcementSummary`) and updated types need schema registration.

### Pattern 5: `GateCriterion` -> `Criterion` conversion via `to_criterion()`

The `to_criterion()` function in `crates/assay-core/src/gate/mod.rs:229-237` maps `GateCriterion` fields to `Criterion`. When `enforcement` is added to both types, this function must propagate it:

```rust
// Current (crates/assay-core/src/gate/mod.rs:229-237)
pub fn to_criterion(gc: &GateCriterion) -> Criterion {
    Criterion {
        name: gc.name.clone(),
        description: gc.description.clone(),
        cmd: gc.cmd.clone(),
        path: gc.path.clone(),
        timeout: gc.timeout,
    }
}
```

### Pattern 6: Evaluation flow (evaluate_all / evaluate_all_gates)

Both `evaluate_all()` and `evaluate_all_gates()` in `crates/assay-core/src/gate/mod.rs` follow the same structure:

1. Iterate criteria
2. Skip criteria with no `cmd` and no `path` (increment `skipped`)
3. Evaluate each criterion via `evaluate()`
4. Increment `passed` or `failed` based on `gate_result.passed`
5. Build `GateRunSummary` with counts

**Key insertion points for enforcement:**
- After step 2 (skip check): skipped criteria bypass enforcement entirely
- At step 4: instead of just `passed += 1` / `failed += 1`, also track enforcement-specific counts
- At step 5: compute `EnforcementSummary` and new `passed` boolean

### Pattern 7: Spec-level defaults via a `[gate]` section

Currently, neither `Spec` nor `GatesSpec` has a `[gate]` section. The decision calls for:

```toml
[gate]
enforcement = "required"
```

This maps to a new struct:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GateConfig {
    #[serde(default)]
    pub enforcement: Enforcement,
}
```

Both `Spec` and `GatesSpec` get an optional `gate` field:

```rust
// On Spec:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub gate: Option<GateConfig>,

// On GatesSpec:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub gate: Option<GateConfig>,
```

**TOML parsing note:** `[gate]` is a TOML table that maps to a struct field named `gate`. The `toml` crate handles this naturally -- a `[gate]` section in TOML maps to a field named `gate` on the parent struct.

### Pattern 8: Validation happens in `assay-core/src/spec/mod.rs`

The validation functions `validate()`, `validate_gates_spec()`, and `validate_feature_spec()` collect all errors into a `Vec<SpecError>` and return them. The "at least one required criterion" check goes here:

```rust
// In validate_gates_spec(), after existing criteria validation:
let required_count = spec.criteria.iter()
    .filter(|c| {
        // Only count executable criteria
        (c.cmd.is_some() || c.path.is_some()) &&
        // Resolve enforcement: criterion override > gate default > Required
        c.enforcement.unwrap_or_else(|| {
            spec.gate.as_ref()
                .map(|g| g.enforcement)
                .unwrap_or(Enforcement::Required)
        }) == Enforcement::Required
    })
    .count();

if required_count == 0 {
    errors.push(SpecError {
        field: "criteria".into(),
        message: "at least one criterion must have enforcement = \"required\"".into(),
    });
}
```

---

## Don't Hand-Roll

1. **Serde enum deserialization** -- Use `#[serde(rename_all = "kebab-case")]` for the `Enforcement` enum. Do NOT implement custom `Deserialize`.

2. **Custom error messages for invalid enforcement values** -- The CONTEXT.md says "Parse errors include helpful suggestion of closest valid value." This is tricky with serde derive alone. The `toml` crate's error messages already include `expected one of "required", "advisory"` when using `rename_all`. However, for a "did you mean?" suggestion, you'd need a custom deserializer. **Recommendation:** Start with derive-only. The toml crate's default error message for an unrecognized enum variant is already helpful (it lists valid values). Only add a custom deserializer if the default message is deemed insufficient during review.

3. **Schema generation** -- Use `schemars::JsonSchema` derive. Do NOT hand-write JSON schemas.

4. **Default value computation** -- Use `#[serde(default)]` and `impl Default`. Do NOT compute defaults at parse time with manual code when serde can do it.

---

## Common Pitfalls

### Pitfall 1: Breaking backward compat on `Spec` (legacy flat files)

Legacy `Spec` uses `#[serde(deny_unknown_fields)]`. Adding `gate: Option<GateConfig>` with `#[serde(default)]` is safe -- existing files without `[gate]` will deserialize with `gate: None`. But the `#[serde(default)]` is **required** or old files will fail.

### Pitfall 2: Enforcement field on `Criterion` vs resolved enforcement

The `enforcement` field on `Criterion` and `GateCriterion` should be `Option<Enforcement>`, NOT `Enforcement`. This is because:
- `None` means "use the spec-level default" (from `[gate]` section)
- `Some(Required)` means explicit override
- `Some(Advisory)` means explicit override

Resolution happens at evaluation time, not at the type level. The resolved enforcement (always `Required` or `Advisory`, never `None`) goes into `CriterionResult.enforcement`.

### Pitfall 3: Skipped criteria enforcement resolution

Decision: "Skipped criteria are always advisory regardless of declared enforcement -- they go in the skipped bucket only, never in enforcement counts."

This means:
- Skipped criteria (`cmd.is_none() && path.is_none()`) increment `skipped` count only
- They do NOT appear in `EnforcementSummary` counts at all
- Their `CriterionResult.enforcement` field should still reflect their declared/resolved enforcement (for informational purposes), but it must NOT influence the pass/fail calculation

### Pitfall 4: The pass formula and its edge cases

`passed = (required_passed == required_total)` where `required_total` counts only **executable** required criteria. A gate with:
- 2 required criteria, 1 passes, 1 fails: `passed = false` (1 != 2)
- 2 required criteria, 2 pass, 3 advisory fail: `passed = true` (2 == 2)
- 1 required (skipped), 1 advisory (cmd): parse-time error (zero executable required)

### Pitfall 5: Duplication between `evaluate_all` and `evaluate_all_gates`

These two functions are near-identical (raised in open issues). When adding enforcement logic, changes must be applied to BOTH. Consider extracting a shared helper, but scope that carefully -- it could be a separate refactor.

### Pitfall 6: `GateRunSummary.passed` field semantics change

Currently, `passed` is a count (`usize`). It is NOT a boolean. The pass/fail boolean is computed by the CLI:
- `print_gate_summary()` checks `counters.failed > 0`
- JSON output: `summary.failed > 0` triggers exit code 1

The new pass formula `passed = (required_passed == required_total)` needs a **new boolean field** on `GateRunSummary` or a change in how the CLI computes pass/fail. Looking at the CONTEXT.md decision, `GateRunSummary` should have the enforcement summary, and the CLI/MCP compute pass/fail from it.

**Wait -- re-reading more carefully:** The `GateRunSummary` has `passed: usize` (a count), but the CLI treats `failed > 0` as the pass/fail signal. The new formula says `passed = (required_passed == required_total)`. This means:
- Old behavior: exit 1 if `failed > 0`
- New behavior: exit 1 if `required_failed > 0` (or equivalently, `required_passed < required_total`)

The `GateRunSummary.passed` (count), `.failed` (count), and `.skipped` (count) fields remain for backward compat. The new `enforcement: EnforcementSummary` carries the enforcement-specific breakdown. The CLI switches to using enforcement counts for exit code determination.

### Pitfall 7: `to_criterion()` must carry enforcement

`to_criterion()` strips `GateCriterion` down to `Criterion`. Both types need the `enforcement: Option<Enforcement>` field, and `to_criterion()` must copy it.

### Pitfall 8: Deprecation warning for specs without `[gate]` section

Decision: "Specs without enforcement field default to required but emit a deprecation warning."

This warning should happen at **load time** (in `load_spec_entry()` or the scan functions), not at evaluation time. The warning text is Claude's discretion.

---

## Code Examples

### New types in `assay-types`

**File: `crates/assay-types/src/enforcement.rs` (new file)**

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Enforcement level for a gate criterion.
///
/// Determines whether a criterion failure blocks the gate (required)
/// or is informational only (advisory).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Enforcement {
    /// Failure blocks the gate. This is the default.
    #[default]
    Required,
    /// Failure is informational; does not block the gate.
    Advisory,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "enforcement",
        generate: || schemars::schema_for!(Enforcement),
    }
}

/// Gate-level configuration section.
///
/// Parsed from `[gate]` in spec TOML files. Provides spec-wide defaults
/// that individual criteria can override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GateSection {
    /// Default enforcement level for all criteria in this spec.
    #[serde(default)]
    pub enforcement: Enforcement,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-section",
        generate: || schemars::schema_for!(GateSection),
    }
}

/// Enforcement breakdown in a gate run summary.
///
/// Always present on `GateRunSummary`, with counts defaulting to 0.
/// Only counts executable criteria (skipped criteria are excluded).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EnforcementSummary {
    pub required_passed: usize,
    pub required_failed: usize,
    pub advisory_passed: usize,
    pub advisory_failed: usize,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "enforcement-summary",
        generate: || schemars::schema_for!(EnforcementSummary),
    }
}
```

### Adding enforcement to Criterion

**File: `crates/assay-types/src/criterion.rs`**

Add after the `timeout` field:

```rust
    /// Enforcement level for this criterion. `None` means "use the spec-level
    /// default from `[gate]` section" (which itself defaults to `required`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub enforcement: Option<Enforcement>,
```

### Adding enforcement to GateCriterion

Same field added to `GateCriterion` in `crates/assay-types/src/gates_spec.rs`.

### Adding `[gate]` section to Spec and GatesSpec

**On `Spec` (`crates/assay-types/src/lib.rs`):**

```rust
    /// Gate configuration section (enforcement defaults).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateSection>,
```

**On `GatesSpec` (`crates/assay-types/src/gates_spec.rs`):**

```rust
    /// Gate configuration section (enforcement defaults).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateSection>,
```

### Adding enforcement to CriterionResult

**File: `crates/assay-types/src/gate_run.rs`**

```rust
pub struct CriterionResult {
    pub criterion_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<GateResult>,
    /// Resolved enforcement level for this criterion.
    #[serde(default)]
    pub enforcement: Enforcement,
}
```

### Adding EnforcementSummary to GateRunSummary

```rust
pub struct GateRunSummary {
    pub spec_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<CriterionResult>,
    pub passed: usize,      // backward compat
    pub failed: usize,      // backward compat
    pub skipped: usize,     // backward compat
    pub total_duration_ms: u64,
    /// Enforcement-level breakdown of results.
    #[serde(default)]
    pub enforcement: EnforcementSummary,
}
```

### Enforcement resolution helper

**File: `crates/assay-core/src/gate/mod.rs`**

```rust
/// Resolve the effective enforcement level for a criterion.
///
/// Precedence: per-criterion override > spec-level `[gate]` default > Required.
fn resolve_enforcement(
    criterion_enforcement: Option<Enforcement>,
    gate_section: Option<&GateSection>,
) -> Enforcement {
    criterion_enforcement.unwrap_or_else(|| {
        gate_section
            .map(|g| g.enforcement)
            .unwrap_or(Enforcement::Required)
    })
}
```

### Updated evaluate_all sketch

```rust
pub fn evaluate_all(
    spec: &Spec,
    working_dir: &Path,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> GateRunSummary {
    let start = Instant::now();
    let mut results = Vec::with_capacity(spec.criteria.len());
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut enforcement_summary = EnforcementSummary::default();

    for criterion in &spec.criteria {
        let resolved_enforcement = resolve_enforcement(
            criterion.enforcement,
            spec.gate.as_ref(),
        );

        if criterion.cmd.is_none() && criterion.path.is_none() {
            skipped += 1;
            results.push(CriterionResult {
                criterion_name: criterion.name.clone(),
                result: None,
                enforcement: resolved_enforcement,
            });
            continue;
        }

        let timeout = resolve_timeout(cli_timeout, criterion.timeout, config_timeout);

        match evaluate(criterion, working_dir, timeout) {
            Ok(gate_result) => {
                if gate_result.passed {
                    passed += 1;
                    match resolved_enforcement {
                        Enforcement::Required => enforcement_summary.required_passed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_passed += 1,
                    }
                } else {
                    failed += 1;
                    match resolved_enforcement {
                        Enforcement::Required => enforcement_summary.required_failed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_failed += 1,
                    }
                }
                results.push(CriterionResult {
                    criterion_name: criterion.name.clone(),
                    result: Some(gate_result),
                    enforcement: resolved_enforcement,
                });
            }
            Err(err) => {
                failed += 1;
                match resolved_enforcement {
                    Enforcement::Required => enforcement_summary.required_failed += 1,
                    Enforcement::Advisory => enforcement_summary.advisory_failed += 1,
                }
                results.push(CriterionResult {
                    criterion_name: criterion.name.clone(),
                    result: Some(GateResult { /* error result */ }),
                    enforcement: resolved_enforcement,
                });
            }
        }
    }

    GateRunSummary {
        spec_name: spec.name.clone(),
        results,
        passed,
        failed,
        skipped,
        total_duration_ms: start.elapsed().as_millis() as u64,
        enforcement: enforcement_summary,
    }
}
```

### CLI pass/fail logic change

**File: `crates/assay-cli/src/main.rs`**

Currently the `StreamCounters` and `print_gate_summary` check `counters.failed > 0`. With enforcement:

```rust
struct StreamCounters {
    passed: usize,
    failed: usize,
    skipped: usize,
    required_passed: usize,
    required_total: usize,  // required_passed + required_failed
}
```

And in `print_gate_summary`:
```rust
// Old: if counters.failed > 0 { std::process::exit(1); }
// New: if counters.required_passed < counters.required_total { std::process::exit(1); }
```

For JSON mode, the same logic uses `summary.enforcement.required_failed > 0`.

### Validation: at least one required criterion

In `validate_gates_spec()`:

```rust
// After existing criteria validation, before returning:
let has_required_executable = spec.criteria.iter().any(|c| {
    let is_executable = c.cmd.is_some() || c.path.is_some();
    let enforcement = c.enforcement.unwrap_or_else(|| {
        spec.gate.as_ref()
            .map(|g| g.enforcement)
            .unwrap_or(Enforcement::Required)
    });
    is_executable && enforcement == Enforcement::Required
});

if !has_required_executable {
    errors.push(SpecError {
        field: "criteria".into(),
        message: "at least one executable criterion must have enforcement = \"required\"; a gate with only advisory criteria would always pass".into(),
    });
}
```

The same check applies to `validate()` (for legacy `Spec`).

---

## Schema Generation Implications

- **3 new types** need `inventory::submit!`: `Enforcement`, `GateSection`, `EnforcementSummary`
- **4 existing types** will have schema changes: `Criterion`, `GateCriterion`, `Spec`, `GatesSpec`, `CriterionResult`, `GateRunSummary`
- Snapshot tests (if any exist for schemas) will need updating
- The `schemars` `JsonSchema` derive handles `Option<Enforcement>` correctly -- it generates a nullable schema with the enum's oneOf

---

## Key File Change Map

| File | Changes |
|------|---------|
| `crates/assay-types/src/enforcement.rs` | **NEW** -- `Enforcement` enum, `GateSection`, `EnforcementSummary` |
| `crates/assay-types/src/lib.rs` | Add `pub mod enforcement;` + re-exports + `gate: Option<GateSection>` on `Spec` |
| `crates/assay-types/src/criterion.rs` | Add `enforcement: Option<Enforcement>` field |
| `crates/assay-types/src/gates_spec.rs` | Add `enforcement: Option<Enforcement>` on `GateCriterion`, `gate: Option<GateSection>` on `GatesSpec` |
| `crates/assay-types/src/gate_run.rs` | Add `enforcement: Enforcement` on `CriterionResult`, `enforcement: EnforcementSummary` on `GateRunSummary` |
| `crates/assay-core/src/gate/mod.rs` | `resolve_enforcement()` helper, update `evaluate_all()` and `evaluate_all_gates()`, update `to_criterion()` |
| `crates/assay-core/src/spec/mod.rs` | "At least one required" validation in `validate()` and `validate_gates_spec()`, deprecation warning in load functions |
| `crates/assay-cli/src/main.rs` | Update `StreamCounters`, `stream_criterion()`, `print_gate_summary()`, JSON exit code logic |

---

## Open Questions for Planner

1. **`evaluate_all` / `evaluate_all_gates` duplication:** Both need identical enforcement changes. Should this phase extract a shared helper, or apply changes to both independently? (The open issue `evaluate-all-duplication` exists but is separate scope.)

2. **`spec new` template:** The `handle_spec_new()` function generates template TOML. Should it include `[gate]\nenforcement = "required"` in the template? (Decision says "[gate] required for new specs".)

3. **MCP server impact:** The MCP `gate_run` tool returns `GateRunSummary` as JSON. The new `enforcement` field will appear automatically via serde. No MCP-specific code changes needed, but the tool description should mention enforcement. Is that in scope?

---

*Research complete: 2026-03-04*
*Confidence: HIGH -- all findings from direct codebase analysis*
