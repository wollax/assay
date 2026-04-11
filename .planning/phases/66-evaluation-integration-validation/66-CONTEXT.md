# Phase 66: Evaluation Integration + Validation - Context

**Gathered:** 2026-04-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire resolution into gate evaluation, precondition enforcement, and `spec_validate` composability diagnostics. Gate evaluation runs resolved (flattened) criteria through the existing evaluator. Precondition checks gate execution before criteria run. `PreconditionFailed` is a distinct non-failure result. `spec_validate` reports composability errors (missing parents, missing libraries, cycles, invalid slugs) and precondition reference errors.

</domain>

<decisions>
## Implementation Decisions

### Resolution integration
- Caller pre-resolves: CLI/MCP handlers call `compose::resolve()` first, then pass `ResolvedGate`'s criteria to a new `evaluate_all_resolved()` function
- `evaluate_all_resolved()` accepts `&[ResolvedCriterion]` (with source annotations) — enables INHR-04 per-criterion source in output
- `CriterionResult` gains an optional `source: Option<CriterionSource>` field with `#[serde(default, skip_serializing_if)]` — backward compatible
- Claude's discretion on whether existing `evaluate_all(spec, ...)` stays as-is for non-composed specs or is refactored to use resolved internally

### Precondition result flow
- New `GateEvalOutcome` enum: `Evaluated(GateRunSummary)` | `PreconditionFailed(PreconditionStatus)` — clean separation, precondition failure means criteria never ran
- `check_preconditions()` is a separate function — callers orchestrate: check preconditions → if passed → resolve → evaluate
- `check_preconditions()` takes a closure `impl Fn(&str) -> Option<bool>` for `requires` lookup — consistent with zero-trait convention
- `GateEvalOutcome` stored directly in run history — history queries can distinguish precondition failures from criteria failures

### Precondition command execution
- Reuse existing `evaluate_command()` infrastructure (spawn, timeout, kill, output capture, head+tail truncation)
- Same timeout as gate criteria — existing timeout resolution chain (CLI flag → spec-level → config → 30s default)
- No gate history = not passed (conservative) — `last_gate_passed(slug).unwrap_or(false)`
- Evaluation order: requires first (cheap history lookups), then commands (expensive shell execution) — all evaluated, no short-circuit

### spec_validate composability diagnostics
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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `gate::evaluate_command()` (`crates/assay-core/src/gate/mod.rs:731`): Spawn/timeout/kill/truncation — reuse for precondition commands
- `gate::evaluate_all()` (`crates/assay-core/src/gate/mod.rs:151`): Base for new `evaluate_all_resolved()`
- `compose::resolve()` (`crates/assay-core/src/spec/compose.rs:243`): Closure-based resolution — callers invoke before evaluate
- `validate_spec_with_dependencies()` (`crates/assay-core/src/spec/validate.rs:315`): Already loads all specs for cycle detection — extend for composability checks
- `enriched_error_display` (`crates/assay-core/src/gate/mod.rs:474`): Fuzzy matching pattern for suggestions
- `PreconditionStatus`, `RequireStatus`, `CommandStatus` (`crates/assay-types/src/precondition.rs`): Types already exist from Phase 64

### Established Patterns
- Free functions with closures, zero-trait convention
- `#[serde(default, skip_serializing_if)]` on all optional fields — CriterionResult.source follows this
- `GateRunSummary` returned from evaluate functions — `GateEvalOutcome` wraps it
- Diagnostic model: `ValidationResult` with `Vec<Diagnostic>` and `DiagnosticSummary` in spec/validate.rs

### Integration Points
- `gate/mod.rs`: New `evaluate_all_resolved()` function, new `check_preconditions()` function
- `assay-types`: `GateEvalOutcome` enum, `CriterionResult` gains `source` field
- `spec/validate.rs`: `validate_spec_with_dependencies()` gains composability + precondition checks
- `assay-cli/src/commands/spec.rs` and `assay-mcp/src/server.rs`: Callers updated to use pre-resolve → evaluate pipeline
- Run history: storage format changes to accommodate `GateEvalOutcome`

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Follow existing assay-core conventions throughout.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 66-evaluation-integration-validation*
*Context gathered: 2026-04-11*
