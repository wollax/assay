# Phase 66: Evaluation Integration + Validation - Research

**Researched:** 2026-04-11
**Domain:** Rust gate evaluation pipeline integration, precondition checking, spec validation diagnostics
**Confidence:** HIGH

## Summary

Phase 66 wires the composition result from Phase 65 into the actual gate evaluation pipeline and adds two safety layers. The work is almost entirely internal refactoring and extension of existing, well-tested Rust code. There is no external library research needed — every API called in this phase already exists in the codebase; the task is to write new functions that stitch them together.

The three deliverable areas are independent and can be implemented in any order: (1) `evaluate_all_resolved()` in `gate/mod.rs` that accepts `&[ResolvedCriterion]`; (2) `check_preconditions()` in `gate/mod.rs` with a `GateEvalOutcome` wrapper type in `assay-types`; and (3) composability + precondition diagnostics wired into `validate_spec_with_dependencies()` in `spec/validate.rs`.

**Primary recommendation:** Implement in three sequential commits — types first (`GateEvalOutcome`, `CriterionResult.source`), then precondition checking, then evaluation integration + validation diagnostics — to keep each diff reviewable and tests focused.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Caller pre-resolves: CLI/MCP handlers call `compose::resolve()` first, then pass `ResolvedGate`'s criteria to a new `evaluate_all_resolved()` function
- `evaluate_all_resolved()` accepts `&[ResolvedCriterion]` (with source annotations) — enables INHR-04 per-criterion source in output
- `CriterionResult` gains an optional `source: Option<CriterionSource>` field with `#[serde(default, skip_serializing_if)]` — backward compatible
- New `GateEvalOutcome` enum: `Evaluated(GateRunSummary)` | `PreconditionFailed(PreconditionStatus)` — clean separation, precondition failure means criteria never ran
- `check_preconditions()` is a separate function — callers orchestrate: check preconditions → if passed → resolve → evaluate
- `check_preconditions()` takes a closure `impl Fn(&str) -> Option<bool>` for `requires` lookup — consistent with zero-trait convention
- `GateEvalOutcome` stored directly in run history — history queries can distinguish precondition failures from criteria failures
- Reuse existing `evaluate_command()` infrastructure for precondition commands
- Same timeout as gate criteria — existing timeout resolution chain (CLI flag → spec-level → config → 30s default)
- No gate history = not passed (conservative) — `last_gate_passed(slug).unwrap_or(false)`
- Evaluation order: requires first (cheap history lookups), then commands (expensive shell execution) — all evaluated, no short-circuit
- **Errors (block valid=true):** missing parent gate, missing library, invalid slug in extends/include (SAFE-02), cycle in extends chain
- **Warnings:** shadow warning (own criterion overrides parent/library), empty includes list (no-op)
- Load external files during validation — `validate_spec_with_dependencies()` already loads all specs; extend to load parent gate and libraries, call `resolve()` for shadow detection
- Precondition references validated too: requires slugs pass `validate_slug()` and exist in specs_dir, commands are non-empty, self-referencing requires warned
- Fuzzy suggestions on missing parent/library (reuse existing enriched_error_display pattern)

### Claude's Discretion
- Whether existing `evaluate_all()` stays as-is or gets refactored to use resolved path internally
- Exact GateEvalOutcome serde representation (tagged enum style)
- How shadow detection identifies overridden criteria (by name comparison during resolve)
- How validate_spec_with_dependencies() receives paths to gate files and library directory

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PREC-01 | User can define `[preconditions].requires` — gate skipped unless named spec's last gate run passed | `check_preconditions()` checks `history::list()` + `history::load()` for the named slug; `GateEvalOutcome::PreconditionFailed` returned when any require fails |
| PREC-02 | User can define `[preconditions].commands` — shell commands that must succeed before gate evaluation | Reuse `evaluate_command()` infrastructure in new `check_preconditions()` function |
| PREC-03 | Precondition failures produce distinct `PreconditionFailed` result (blocked != failed) | `GateEvalOutcome` enum in `assay-types` with `Evaluated(GateRunSummary)` vs `PreconditionFailed(PreconditionStatus)` variants |
| SAFE-01 | `spec_validate` detects composability errors (missing parents, missing libraries, cycle detection) | Extend `validate_spec_with_dependencies()` to call `compose::resolve()` and convert errors to `Diagnostic` entries |
| SAFE-02 | `extends` and `include` values are slug-validated to prevent path traversal | `compose::validate_slug()` already enforces ASCII-only slug chars; call it in validation before any file I/O |
</phase_requirements>

## Standard Stack

### Core
| Item | Location | Purpose |
|------|----------|---------|
| `gate::evaluate_command()` | `crates/assay-core/src/gate/mod.rs:731` | Spawn/timeout/kill/output-capture — reuse for precondition commands |
| `gate::evaluate_all()` | `crates/assay-core/src/gate/mod.rs:151` | Base pattern for new `evaluate_all_resolved()` |
| `gate::evaluate_criteria()` (private) | `crates/assay-core/src/gate/mod.rs:279` | Shared inner loop — `evaluate_all_resolved()` will follow same pattern |
| `gate::resolve_timeout()` | `crates/assay-core/src/gate/mod.rs:655` | CLI → criterion → config → 300s chain; use for precondition commands |
| `gate::resolve_enforcement()` | `crates/assay-core/src/gate/mod.rs:640` | Used by evaluate_all; same pattern needed in evaluate_all_resolved |
| `compose::resolve()` | `crates/assay-core/src/spec/compose.rs:243` | Closure-based resolution — callers invoke before evaluate |
| `compose::validate_slug()` | `crates/assay-core/src/spec/compose.rs:21` | Validates slug chars; call in validation for SAFE-02 |
| `validate_spec_with_dependencies()` | `crates/assay-core/src/spec/validate.rs:315` | Already loads all specs; extend for composability checks |
| `PreconditionStatus` / `RequireStatus` / `CommandStatus` | `crates/assay-types/src/precondition.rs` | All already defined — Phase 64 output |
| `history::list()` + `history::load()` | `crates/assay-core/src/history/mod.rs` | Read recorded runs; use to implement `last_gate_passed()` |
| `CriterionSource` | `crates/assay-types/src/resolved_gate.rs` | Already has `Own`, `Parent { gate_slug }`, `Library { slug }` variants |
| `ResolvedCriterion` | `crates/assay-types/src/resolved_gate.rs` | `{ criterion: Criterion, source: CriterionSource }` — input to `evaluate_all_resolved()` |

No new crate dependencies needed. All required infrastructure was built in phases 64–65.

## Architecture Patterns

### Recommended Code Structure

```
assay-types/src/gate_run.rs          — add CriterionResult.source field
assay-types/src/gate_eval_outcome.rs — new: GateEvalOutcome enum (or add to gate_run.rs)
assay-core/src/gate/mod.rs           — add evaluate_all_resolved(), check_preconditions()
assay-core/src/history/mod.rs        — add last_gate_passed() helper
assay-core/src/spec/validate.rs      — extend validate_spec_with_dependencies()
assay-cli/src/commands/gate.rs       — update handle_gate_run() callers
```

### Pattern 1: GateEvalOutcome enum (assay-types)

**What:** A tagged enum wrapping either a completed evaluation or a precondition block. Stored in history so queries can distinguish the two outcomes.

**When to use:** Returned by any function that may refuse to evaluate due to preconditions.

```rust
// In assay-types — tagged enum, externally tagged (serde default)
// Or use internally tagged: #[serde(tag = "outcome", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GateEvalOutcome {
    Evaluated(GateRunSummary),
    PreconditionFailed(PreconditionStatus),
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "gate-eval-outcome",
        generate: || schemars::schema_for!(GateEvalOutcome),
    }
}
```

**Discretion note:** The CONTEXT.md leaves serde tag style to Claude's discretion. Recommend `#[serde(tag = "outcome", rename_all = "snake_case")]` (internally tagged) so JSON consumers can discriminate on a stable `"outcome"` field without needing to detect which enum variant key is present. This is consistent with how `CriterionSource` uses `rename_all = "snake_case"`.

### Pattern 2: CriterionResult.source (backward compatible)

**What:** Add `source: Option<CriterionSource>` to `CriterionResult`. `None` = no source info (old records, non-resolved path). Omitted from JSON via `skip_serializing_if`.

```rust
// In assay-types/src/gate_run.rs — add field
pub struct CriterionResult {
    pub criterion_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<GateResult>,
    #[serde(default)]
    pub enforcement: Enforcement,
    // NEW: phase 66
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<CriterionSource>,
}
```

**Critical:** `GateRunRecord` has `#[serde(deny_unknown_fields)]`. Since `source` is added to `CriterionResult` (which is nested inside `GateRunSummary` which is inside `GateRunRecord`), the `deny_unknown_fields` on `GateRunRecord` applies to the record's own fields, not nested types. The nested `CriterionResult` does NOT have `deny_unknown_fields`, so old records without `source` deserialize fine (field defaults to `None`). No migration needed.

### Pattern 3: evaluate_all_resolved() (gate/mod.rs)

**What:** New public function that accepts `&[ResolvedCriterion]` and runs the criteria through `evaluate_criteria()`.

```rust
pub fn evaluate_all_resolved(
    spec_name: &str,
    resolved: &[ResolvedCriterion],
    working_dir: &Path,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> GateRunSummary {
    let criteria: Vec<(Criterion, Enforcement)> = resolved
        .iter()
        .map(|rc| {
            // enforcement defaults to Required when not set (no gate section passed here)
            // caller should pass gate section if available, or resolve independently
            let enforcement = resolve_enforcement(rc.criterion.enforcement, None);
            (rc.criterion.clone(), enforcement)
        })
        .collect();
    // Then build results with source annotation by correlating back to resolved[]
    // OR: thread source through evaluate_criteria (internal refactor)
    ...
}
```

**Discretion point:** `evaluate_criteria()` currently returns `CriterionResult` without `source`. To populate `source` in each result, either:
- (a) Refactor `evaluate_criteria` to accept `&[(Criterion, Enforcement, Option<CriterionSource>)]` — preferred: keeps logic together, one pass
- (b) Post-process results by zipping with `resolved` by index — simpler but fragile if order changes

**Recommendation:** Option (a) — add `source: Option<CriterionSource>` parameter to the private `evaluate_criteria` function and set it on each `CriterionResult`. Existing callers pass `None` per entry. No public API changes.

Also pass the `gate_section` from the loaded `GatesSpec` for correct enforcement defaults.

### Pattern 4: check_preconditions() (gate/mod.rs)

**What:** New function that runs all `requires` and `commands` checks. Returns `PreconditionStatus`.

```rust
pub fn check_preconditions(
    preconditions: &SpecPreconditions,
    last_gate_passed: impl Fn(&str) -> Option<bool>,
    working_dir: &Path,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> PreconditionStatus {
    // 1. requires: all evaluated, no short-circuit
    let requires: Vec<RequireStatus> = preconditions.requires.iter().map(|slug| {
        let passed = last_gate_passed(slug).unwrap_or(false);
        RequireStatus { spec_slug: slug.clone(), passed }
    }).collect();

    // 2. commands: all evaluated, no short-circuit
    let timeout = resolve_timeout(cli_timeout, None, config_timeout);
    let commands: Vec<CommandStatus> = preconditions.commands.iter().map(|cmd| {
        match evaluate_command(cmd, working_dir, timeout) {
            Ok(result) => CommandStatus {
                command: cmd.clone(),
                passed: result.passed,
                output: Some(/* combined stdout+stderr */),
            },
            Err(_) => CommandStatus {
                command: cmd.clone(),
                passed: false,
                output: Some("spawn failed".to_string()),
            },
        }
    }).collect();

    PreconditionStatus { requires, commands }
}
```

**Caller orchestration pattern:**

```rust
// In CLI/MCP handler
let precondition_outcome = if let Some(preconds) = &gates.preconditions {
    let status = gate::check_preconditions(
        preconds,
        |slug| history::last_gate_passed(&assay_dir, slug),
        &working_dir,
        cli_timeout,
        config_timeout,
    );
    let all_passed = status.requires.iter().all(|r| r.passed)
        && status.commands.iter().all(|c| c.passed);
    if !all_passed {
        return GateEvalOutcome::PreconditionFailed(status);
    }
    Some(status) // passed — continue to resolve → evaluate
} else {
    None
};
```

### Pattern 5: last_gate_passed() helper (history/mod.rs)

**What:** New public function that reads the latest run record for a spec and returns whether it passed.

```rust
pub fn last_gate_passed(assay_dir: &Path, spec_name: &str) -> Option<bool> {
    let ids = list(assay_dir, spec_name).ok()?;
    let latest_id = ids.last()?;
    let record = load(assay_dir, spec_name, latest_id).ok()?;
    // "passed" = no required failures
    Some(record.summary.enforcement.required_failed == 0)
}
```

**Edge case:** Specs that have only ever had `PreconditionFailed` outcomes stored (once `GateEvalOutcome` is stored in history). The history storage format must accommodate `GateEvalOutcome`, not just `GateRunSummary`. This is a significant consideration — see Pitfall 1 below.

### Pattern 6: validate_spec_with_dependencies() extension (spec/validate.rs)

**What:** Extend the existing function to check composability and precondition references. The function already receives `specs_dir` and loads all specs. Needs an additional `assay_dir` (or derived paths) to:
- Load parent gates from the gates directory
- Load libraries from `assay_dir/criteria/`

```rust
// Extension to validate_spec_with_dependencies():
if let SpecEntry::Directory { gates, slug, .. } = entry {
    // SAFE-02: validate slugs
    if let Some(extends) = &gates.extends {
        if let Err(e) = compose::validate_slug(extends) {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                location: "extends".to_string(),
                message: e.to_string(),
            });
        } else {
            // SAFE-01: check parent exists
            match load_gate_by_slug(specs_dir, extends) {
                Err(AssayError::ParentGateNotFound { .. }) => {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        location: "extends".to_string(),
                        message: format!("parent gate `{extends}` not found"),
                        // + fuzzy suggestion from available gate slugs
                    });
                }
                // cycle via resolve() call ...
            }
        }
    }
    // Similar for include[] → library validation
    // Precondition requires slugs: validate_slug() + exists in specs_dir
}
```

**Discretion on function signature:** `validate_spec_with_dependencies()` currently takes `entry` and `specs_dir`. To access libraries, it needs `assay_dir` (parent of `specs_dir` and `criteria/`). Options:
- Add `assay_dir: &Path` parameter — straightforward, callers already have it
- Derive from `specs_dir.parent()` — fragile if layout changes
- Recommendation: Add `assay_dir: Option<&Path>` — `None` skips composability checks (backward compat), `Some(dir)` enables them.

### Pattern 7: Shadow detection (warning)

When `resolve()` is called during validation, shadow detection is implicit: the dedup algorithm produces a `ResolvedGate` where own-wins criteria have replaced inherited ones. To emit a warning, compare the resolved criteria set against what would have been there without the own criteria. Simplest approach: after calling `resolve()`, check if any `own` criterion has the same name as any `parent/library` criterion in the resolved set before dedup. This requires a small addition to `resolve()` or a helper that does a pre-dedup comparison.

**Alternative approach (simpler):** Don't call `resolve()` during validation. Instead, manually check: for each own criterion name, is that name also present in the parent gate's criteria list? Avoids the resolution I/O overhead for a warning-only check.

**Recommendation:** Use the simpler manual check — load parent gate criteria names, check intersection with own criteria names. No additional I/O beyond the parent-existence check already needed for SAFE-01.

### Anti-Patterns to Avoid
- **Storing `GateRunSummary` directly in history when `GateEvalOutcome` is used:** Breaks the ability for `last_gate_passed()` to distinguish skipped vs failed runs. Either (a) store `GateEvalOutcome` in `GateRunRecord.summary` (requires field type change) or (b) store a nullable outcome beside the existing `summary` field.
- **Short-circuiting precondition checks:** Context says all preconditions are evaluated, no short-circuit. Don't return early on the first failure.
- **Calling `resolve()` inside `check_preconditions()`:** Resolution and precondition checking are separate concerns. `check_preconditions()` only checks `SpecPreconditions`, not the criteria composition.
- **Passing `assay_dir` through `evaluate_all_resolved()`:** Evaluation doesn't need disk access beyond working_dir. The `last_gate_passed` lookup is the caller's responsibility via closure.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Shell command execution with timeout | Custom subprocess wrapper | `gate::evaluate_command()` — already handles `sh -c`, pipes, timeout, kill, output |
| Slug validation for path-traversal prevention | Custom char check | `compose::validate_slug()` — rejects `.`, `..`, `/`, `\`, path separators at character level |
| Fuzzy suggestions for missing resources | Levenshtein from scratch | `spec::find_fuzzy_match()` + `compose::load_library_by_slug()` already have suggestion logic |
| Run history listing/loading | Manual JSON scanning | `history::list()` + `history::load()` — atomic writes, sorted IDs, path validation |
| Diagnostic accumulation | Custom result type | `Vec<Diagnostic>` + `DiagnosticSummary::from_diagnostics()` — already the pattern in validate.rs |
| Timeout resolution | Per-caller if/else | `gate::resolve_timeout()` — CLI → criterion → config → 300s chain |
| Inventory schema registration | Manual registry | `inventory::submit!` macro — used by every type in assay-types |

## Common Pitfalls

### Pitfall 1: History storage format for GateEvalOutcome
**What goes wrong:** `GateRunRecord` stores `summary: GateRunSummary` directly. If `GateEvalOutcome` should be stored in history, the `GateRunRecord` type needs to change. But `GateRunRecord` has `#[serde(deny_unknown_fields)]` — adding a new field without migration causes all existing run records to fail deserialization.
**Why it happens:** The type was designed before `PreconditionFailed` was a concept.
**How to avoid:** Two options:
- (a) Add `outcome: Option<GateEvalOutcome>` to `GateRunRecord` with `#[serde(default, skip_serializing_if)]` — old records have `outcome: None` and still deserialize fine. `last_gate_passed()` checks both `outcome` and `summary`.
- (b) Don't store precondition failures in history at all (they aren't gate runs). Store only `Evaluated` outcomes. Use in-memory `GateEvalOutcome` only for the return type; persist the inner `GateRunSummary` as before.
**Recommendation:** Option (b) is simpler and matches the semantic: precondition failures are not gate runs, so run history should not contain them. The `GateEvalOutcome` type is the caller's return value; the caller only calls `history::save_run()` for `GateEvalOutcome::Evaluated(summary)`. No `GateRunRecord` schema change needed.
**Warning signs:** If you find yourself adding `GateEvalOutcome` to `GateRunRecord`, reconsider — it creates a schema migration problem.

### Pitfall 2: validate_slug() vs path traversal
**What goes wrong:** `../evil` contains only valid slug chars... no, wait. `validate_slug()` rejects `/` (not in `[a-z0-9-_]`) and rejects `.` at position 0 (must start `[a-z0-9]`). So `../evil` is rejected by slug validation. But `..` alone is also rejected (starts with `.`).
**Why it happens:** Could seem like slug validation isn't sufficient without explicit traversal check.
**How to avoid:** Confirm that `compose::validate_slug()` already handles SAFE-02 by design. The test suite in compose.rs verifies this. No additional traversal check needed — slug validation is sufficient.
**Note:** The existing `history::validate_path_component()` rejects `.`, `..`, `/`, `\` separately — that's for run IDs and spec names. Slugs are stricter: must be `[a-z0-9][a-z0-9-_]*`, which means `.` is never valid as a leading char.

### Pitfall 3: evaluate_all() callers during the refactor
**What goes wrong:** There are multiple callers of `evaluate_all()`, `evaluate_all_gates()`, `evaluate_all_with_events()`. If you change `evaluate_criteria()` (the private shared loop), all callers are affected.
**Why it happens:** The private function is shared by 4 public functions.
**How to avoid:** When adding `source: Option<CriterionSource>` to the `evaluate_criteria` signature, ensure all 4 existing callers pass `None` per criterion. Compile-test immediately. Don't change the public function signatures.

### Pitfall 4: Composability validation requiring assay_dir
**What goes wrong:** `validate_spec_with_dependencies()` currently only takes `specs_dir`. To load libraries (which live in `assay_dir/criteria/`), the function needs `assay_dir`. But `specs_dir` is typically `assay_dir/specs/`, so `specs_dir.parent()` works — but only if the layout is always `<assay_dir>/specs/`.
**Why it happens:** The path relationship is a convention, not enforced by the type.
**How to avoid:** Add an explicit `assay_dir: Option<&Path>` parameter rather than deriving it from `specs_dir.parent()`. The MCP and CLI callers already hold both paths. Skip composability checks when `assay_dir` is `None`.

### Pitfall 5: GateEvalOutcome serde and existing JSON consumers
**What goes wrong:** If the MCP `gate_run` tool currently returns `GateRunSummary` directly, and is changed to return `GateEvalOutcome`, existing MCP consumers break on the shape change.
**Why it happens:** API contract change without versioning.
**How to avoid:** The CONTEXT.md says `GateEvalOutcome` is the internal return type from gate execution functions. The MCP tool handler decides how to present it. For backward compat, the MCP handler can unwrap `Evaluated(summary)` and return the summary as before, or add a top-level discriminator. This is a discretion area — document the choice in code.

### Pitfall 6: Shadow warning false positives for intentional overrides
**What goes wrong:** Shadow warnings fire for every own criterion that matches a parent name — but that's exactly what own-wins is for. If user deliberately overrides `compiles` from the parent, the warning is noise.
**Why it happens:** Warning was designed to catch accidents, but it fires on all overrides.
**How to avoid:** Per CONTEXT.md, shadow warning is a `Severity::Warning` not an error. Users who see it can suppress it by choosing different criterion names. No option to suppress needed for v0.7.0. Just emit it.

## Code Examples

Verified patterns from existing code:

### How to add a new type to assay-types with schema registration
```rust
// Source: assay-types/src/precondition.rs (existing pattern)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PreconditionStatus { ... }

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "precondition-status",
        generate: || schemars::schema_for!(PreconditionStatus),
    }
}
```

### How evaluate_criteria builds CriterionResult
```rust
// Source: gate/mod.rs ~299
results.push(CriterionResult {
    criterion_name: criterion.name.clone(),
    result: Some(gate_result),
    enforcement: *resolved_enforcement,
    // After Phase 66: add source: Some(source.clone()) or None
});
```

### How validate_spec_with_dependencies extends diagnostics
```rust
// Source: spec/validate.rs ~320
let mut result = validate_spec(entry, check_commands);
// ... load deps, build graph, detect_cycles ...
for cd in cycle_diagnostics {
    if cd.specs.contains(slug) {
        result.diagnostics.push(cd.diagnostic);
    }
}
result.summary = DiagnosticSummary::from_diagnostics(&result.diagnostics);
result.valid = result.summary.errors == 0;
```

### How CriterionSource variants serialize
```rust
// Source: assay-types/src/resolved_gate.rs (tests)
// Own → "own"
// Parent { gate_slug: "base" } → {"parent": {"gate_slug": "base"}}
// Library { slug: "rust-basics" } → {"library": {"slug": "rust-basics"}}
// (default externally-tagged serde with rename_all = "snake_case")
```

### How AssayError errors convert to diagnostics in validate
```rust
// Pattern from existing composability error handling:
Err(AssayError::InvalidSlug { slug, reason }) => {
    diagnostics.push(Diagnostic {
        severity: Severity::Error,
        location: "extends".to_string(),
        message: format!("invalid slug `{slug}`: {reason}"),
    });
}
Err(AssayError::ParentGateNotFound { parent_slug, .. }) => {
    diagnostics.push(Diagnostic {
        severity: Severity::Error,
        location: "extends".to_string(),
        message: format!("parent gate `{parent_slug}` not found"),
    });
}
Err(AssayError::CycleDetected { gate_slug, parent_slug }) => {
    diagnostics.push(Diagnostic {
        severity: Severity::Error,
        location: "extends".to_string(),
        message: format!("circular extends: `{gate_slug}` and `{parent_slug}` extend each other"),
    });
}
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `evaluate_all(&spec)` — takes whole Spec | `evaluate_all_resolved(&[ResolvedCriterion])` — takes flattened resolved criteria | Enables composition-aware evaluation with source annotations |
| No precondition enforcement | `check_preconditions() → GateEvalOutcome` | Gate criteria never run when preconditions fail |
| `spec_validate` only checks syntax/cycles in `depends` | Validation also checks `extends`, `include`, and precondition slugs | Catches composability errors at author time |

**Not yet implemented (deferred to future phases):**
- PREC-04: Staleness window for `requires` (last pass within N minutes)
- INHR-05: Multi-level inheritance depth > 1

## Open Questions

1. **History storage for GateEvalOutcome**
   - What we know: `GateRunRecord.summary` is `GateRunSummary`. `deny_unknown_fields` is on `GateRunRecord`, not its nested types.
   - What's unclear: Should precondition failures appear in `.assay/results/<spec>/` at all?
   - Recommendation: No — only save `Evaluated` runs to history. `GateEvalOutcome::PreconditionFailed` is in-memory only. This avoids schema changes and keeps history semantically clean (only actual gate runs).

2. **validate_spec_with_dependencies() signature change**
   - What we know: Currently `(entry, check_commands, specs_dir)`. Needs `assay_dir` for library path.
   - What's unclear: Whether callers in MCP and CLI both have `assay_dir` readily available.
   - Recommendation: Grep for all call sites before changing. Add `assay_dir: Option<&Path>` and skip composability when absent.

3. **evaluate_all_resolved() and gate_section enforcement**
   - What we know: `GatesSpec.gate` holds the default enforcement section. `evaluate_all_resolved` receives `&[ResolvedCriterion]`, not the full `GatesSpec`.
   - What's unclear: Whether callers should pass `Option<&GateSection>` alongside the resolved criteria.
   - Recommendation: Add `gate_section: Option<&GateSection>` to `evaluate_all_resolved()`. Callers pass `gates.gate.as_ref()`. This matches `evaluate_all_gates()` pattern.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` |
| Config file | `Cargo.toml` workspace (no separate test config) |
| Quick run command | `cargo test -p assay-core --lib 2>&1 \| grep -E "test .*(FAILED\|ok\|IGNORED)"` |
| Full suite command | `just test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PREC-01 | Spec with failing `requires` entry returns `PreconditionFailed` | unit | `cargo test -p assay-core --lib gate::tests::precondition_requires_not_passed` | ❌ Wave 0 |
| PREC-01 | Spec with no history for required slug returns `PreconditionFailed` | unit | `cargo test -p assay-core --lib gate::tests::precondition_requires_no_history` | ❌ Wave 0 |
| PREC-02 | Spec with failing `commands` entry returns `PreconditionFailed` | unit | `cargo test -p assay-core --lib gate::tests::precondition_command_fails` | ❌ Wave 0 |
| PREC-03 | `GateEvalOutcome::PreconditionFailed` is distinct from a failed `GateRunSummary` | unit | `cargo test -p assay-types --lib gate_run::tests::gate_eval_outcome_variants` | ❌ Wave 0 |
| PREC-03 | Criteria are NOT evaluated when precondition fails | unit | `cargo test -p assay-core --lib gate::tests::precondition_blocks_criteria` | ❌ Wave 0 |
| SAFE-01 | `spec_validate` returns error diagnostic for missing parent gate | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_missing_parent` | ❌ Wave 0 |
| SAFE-01 | `spec_validate` returns error diagnostic for missing library | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_missing_library` | ❌ Wave 0 |
| SAFE-01 | `spec_validate` returns error diagnostic for cycle in extends | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_cycle_extends` | ❌ Wave 0 |
| SAFE-02 | `../evil` in extends is rejected before file I/O | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_slug_path_traversal` | ❌ Wave 0 |
| SAFE-02 | Valid slug in extends passes slug check | unit | `cargo test -p assay-core --lib spec::validate::tests::composability_valid_slug_passes` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p assay-core --lib 2>&1 | tail -5`
- **Per wave merge:** `just test`
- **Phase gate:** `just ready` (fmt-check + lint + test + deny) before `/kata:verify-work`

### Wave 0 Gaps
- [ ] Tests for `check_preconditions()` in `crates/assay-core/src/gate/mod.rs` (extend existing `#[cfg(test)]` at line 1131)
- [ ] Tests for `GateEvalOutcome` serde in `crates/assay-types/src/gate_run.rs` (or new file)
- [ ] Tests for composability diagnostics in `crates/assay-core/src/spec/validate.rs` (extend existing `#[cfg(test)]` at line 372)
- [ ] Tests for `evaluate_all_resolved()` + `CriterionResult.source` in `crates/assay-core/src/gate/mod.rs`
- [ ] Tests for `history::last_gate_passed()` in `crates/assay-core/src/history/mod.rs` (extend existing `#[cfg(test)]` at line 261)

None of these require new files — all extend existing inline test modules.

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection — all claims verified against actual Rust source files
  - `crates/assay-types/src/gate_run.rs` — `CriterionResult`, `GateRunSummary`, `GateRunRecord`
  - `crates/assay-types/src/precondition.rs` — `PreconditionStatus`, `RequireStatus`, `CommandStatus`, `SpecPreconditions`
  - `crates/assay-types/src/resolved_gate.rs` — `ResolvedGate`, `ResolvedCriterion`, `CriterionSource`
  - `crates/assay-types/src/gates_spec.rs` — `GatesSpec` with `preconditions`, `extends`, `include` fields
  - `crates/assay-core/src/gate/mod.rs` — `evaluate_command()` at line 731, `evaluate_all()` at line 151, `evaluate_criteria()` at line 279, `resolve_timeout()` at line 655, `resolve_enforcement()` at line 640
  - `crates/assay-core/src/spec/compose.rs` — `validate_slug()` at line 21, `resolve()` at line 243
  - `crates/assay-core/src/spec/validate.rs` — `validate_spec_with_dependencies()` at line 315, diagnostic model
  - `crates/assay-core/src/history/mod.rs` — `save_run()`, `save()`, `load()`, `list()`
  - `crates/assay-core/src/error.rs` — full `AssayError` enum including `ParentGateNotFound`, `CycleDetected`, `InvalidSlug`, `LibraryNotFound`

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all types, functions, and file locations verified by reading source
- Architecture: HIGH — all patterns follow directly from existing code conventions; no novel dependencies
- Pitfalls: HIGH — Pitfall 1 (history storage) and Pitfall 3 (callers) are verified by reading `GateRunRecord` definition and `evaluate_criteria` call sites

**Research date:** 2026-04-11
**Valid until:** 2026-05-11 (stable codebase, no external dependencies)
