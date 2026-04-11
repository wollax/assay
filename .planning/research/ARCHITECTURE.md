# Architecture Patterns: Gate Composability

**Domain:** Gate composability primitives + wizard surface
**Researched:** 2026-04-11
**Confidence:** HIGH (all findings derived from direct codebase analysis)

---

## Existing Architecture Baseline

Before describing new components, the relevant existing structures:

### assay-types (DTOs, no logic)

- `GatesSpec` — top-level spec file (`gates.toml`). Fields: `name`, `description`, `gate: Option<GateSection>`, `depends: Vec<String>`, `milestone`, `order`, `criteria: Vec<Criterion>`. Has `#[serde(deny_unknown_fields)]`.
- `Criterion` — single gate criterion. Fields: `name`, `description`, `cmd`, `path`, `timeout`, `enforcement`, `kind: Option<CriterionKind>`, `prompt`, `requirements`, `when`. Has `#[serde(deny_unknown_fields)]`.
- `GateSection` — `[gate]` block. Only field: `enforcement: Enforcement`. Has `#[serde(deny_unknown_fields)]`.
- `Enforcement` — `Required` | `Advisory`.
- `CriterionKind` — `AgentReport` | `EventCount` | `NoToolErrors` (internally tagged with `type` key, snake_case).

### assay-core (domain logic)

- `spec::mod` — `from_str`, `validate`, `scan`, `SpecEntry` (Legacy | Directory), `ScanResult`. Free functions.
- `spec::validate` — structured `Diagnostic` + `ValidationResult` (used by MCP `spec_validate`).
- `gate::mod` — `evaluate(criterion, working_dir, timeout)`, synchronous. Derives `GateKind` from criterion fields.
- `config::mod` — `load(root)`, `save(root, config)`. Free functions.
- `wizard` — `create_from_inputs`, `create_milestone_from_params`, `create_spec_from_params`. Pure, no TTY.

### Binary crates (thin wrappers)

- `assay-cli` → `commands/plan.rs` — TTY wizard via `dialoguer`, delegates to `assay_core::wizard`.
- `assay-mcp` → `server.rs` — `milestone_create`, `spec_create` MCP tools, delegates to `assay_core::wizard`.
- `assay-tui` — exists but gate wizard not yet present.

---

## New Components Needed

### 1. `assay-types`: New types for composability

**`CriteriaLibrary`** — a named, shareable collection of criteria stored at `.assay/criteria/<name>.toml`.

```toml
# .assay/criteria/rust-baseline.toml
name = "rust-baseline"
description = "Standard Rust quality gates"

[[criteria]]
name = "format"
description = "Code is formatted"
cmd = "cargo fmt --check"

[[criteria]]
name = "lint"
description = "No clippy warnings"
cmd = "cargo clippy --workspace -- -D warnings"
```

New type in `assay-types/src/criteria_library.rs`:

```rust
pub struct CriteriaLibrary {
    pub name: String,
    pub description: String,
    pub criteria: Vec<Criterion>,
}
```

**`GateExtends`** — reference to a parent gate definition. Adds `extends: Option<String>` to `GatesSpec` and two new fields to `GateSection`:

- `GatesSpec.extends: Option<String>` — slug of another `GatesSpec` to inherit criteria from.
- `GatesSpec.include: Vec<String>` — list of `CriteriaLibrary` names whose criteria are prepended.

**`SpecPreconditions`** — a new section in `GatesSpec` that lists conditions checked before gate evaluation begins.

```toml
[preconditions]
requires = ["base-setup"]          # other spec slugs that must have passed
commands = ["git diff --quiet"]    # shell commands that must exit 0
```

New type in `assay-types/src/preconditions.rs`:

```rust
pub struct SpecPreconditions {
    pub requires: Vec<String>,    // spec slugs whose gates must have passed
    pub commands: Vec<String>,    // shell commands that must succeed
}
```

**`GatesSpec` field additions** (additive, backward-compatible — all new fields are `Option` or `Vec` with defaults, so existing `gates.toml` files continue to deserialize):

```rust
pub struct GatesSpec {
    // existing fields ...
    pub extends: Option<String>,                      // NEW: inherit criteria from parent spec
    pub include: Vec<String>,                         // NEW: inject named criteria libraries
    pub preconditions: Option<SpecPreconditions>,     // NEW: precondition checks
}
```

Important: `GatesSpec` uses `#[serde(deny_unknown_fields)]`. Adding fields here is a safe additive change — old files deserialize fine because new fields have defaults. Old binaries trying to deserialize new files would fail if the files contain the new keys, but both sides are in this workspace.

---

### 2. `assay-core`: Composability resolution layer

New module: `assay-core/src/spec/compose.rs`

Responsibility: take a raw `GatesSpec` (as loaded from disk, possibly with `extends`/`include`/`preconditions`) and produce a `ResolvedGatesSpec` — a flat, fully-expanded spec ready for evaluation. This is a pure transformation function:

```rust
pub struct ResolvedGatesSpec {
    pub spec: GatesSpec,                   // original, preserved for display
    pub effective_criteria: Vec<Criterion>, // merged: library + parent + own
}

pub fn resolve(
    raw: &GatesSpec,
    load_library: impl Fn(&str) -> Result<CriteriaLibrary>,
    load_parent: impl Fn(&str) -> Result<GatesSpec>,
) -> Result<ResolvedGatesSpec>
```

Merge order for `effective_criteria`: library criteria (in `include` order) then parent's `effective_criteria` (recursively resolved) then own `criteria`.

Cycle detection: track visited slugs during recursive resolution. Return `AssayError::CompositionCycle` on detection.

**`preconditions` evaluation** — separate step from criteria evaluation, runs first:

```rust
pub fn evaluate_preconditions(
    preconditions: &SpecPreconditions,
    passed_gates: &HashSet<String>,   // set of spec slugs with passing gates in history
    working_dir: &Path,
    timeout: Duration,
) -> PreconditionStatus
```

`PreconditionStatus` is a new type in `assay-types`:

```rust
pub struct PreconditionStatus {
    pub all_met: bool,
    pub unmet_requires: Vec<String>,
    pub failed_commands: Vec<(String, i32)>, // (cmd, exit_code)
}
```

---

### 3. `assay-core`: Criteria library I/O

New module: `assay-core/src/criteria_library.rs`

Three free functions (mirrors `config::mod` and `milestone::mod` patterns):

```rust
pub fn load(assay_dir: &Path, name: &str) -> Result<CriteriaLibrary>
pub fn save(assay_dir: &Path, library: &CriteriaLibrary) -> Result<()>
pub fn scan(assay_dir: &Path) -> Result<Vec<CriteriaLibrary>>
```

Storage path: `.assay/criteria/<name>.toml` — a new subdirectory under `.assay/`.

---

### 4. `assay-core/src/wizard.rs`: Gate wizard extension

The existing `wizard.rs` handles milestone + spec creation. New functions added here (not a new module — consistent with existing structure):

```rust
pub struct GateWizardInputs {
    pub spec_slug: String,
    pub extends: Option<String>,
    pub include: Vec<String>,
    pub preconditions: Option<PreconditionsInput>,
    pub criteria: Vec<CriterionInput>,
    pub replace_criteria: bool,  // if true, replaces existing criteria; false appends
}

pub struct PreconditionsInput {
    pub requires: Vec<String>,
    pub commands: Vec<String>,
}

/// Create or update a spec's gates.toml with composability fields.
pub fn apply_gate_wizard(
    inputs: &GateWizardInputs,
    assay_dir: &Path,
    specs_dir: &Path,
) -> Result<PathBuf>
```

`apply_gate_wizard` differs from `create_spec_from_params`:
- Works on an existing spec (update mode) as well as new specs.
- Writes `extends`, `include`, and `preconditions` fields.
- Validates `extends` target exists before writing.
- Validates each `include` library exists before writing.

---

### 5. CLI surface: `assay gate wizard`

New subcommand under `assay gate` in `assay-cli/src/commands/gate.rs`:

```
assay gate wizard [SPEC]     # interactive: pick spec, configure extends/include/preconditions
```

TTY-guarded (same pattern as `assay plan`). Delegates to `assay_core::wizard::apply_gate_wizard` after collecting inputs via `dialoguer`.

Also: `assay criteria list` and `assay criteria new` subcommands for managing libraries.

New file: `assay-cli/src/commands/criteria.rs`.

---

### 6. MCP surface: new tools in `assay-mcp/src/server.rs`

Additive tools (never modify existing tools — established decision):

| Tool | Parameters | Returns |
|------|-----------|---------|
| `gate_wizard` | `spec_slug`, `extends?`, `include[]`, `preconditions?`, `criteria[]` | path to written `gates.toml` |
| `criteria_list` | — | `Vec<CriteriaLibrary>` |
| `criteria_get` | `name` | `CriteriaLibrary` |
| `criteria_create` | `name`, `description`, `criteria[]` | path to written library |
| `spec_resolve` | `spec_slug` | `ResolvedGatesSpec` with `effective_criteria` expanded |

`spec_resolve` is useful for agents to preview the full flattened criteria before running.

---

### 7. TUI surface: `assay-tui`

Gate wizard as a dedicated TUI screen. The TUI currently has a gate results viewer on the roadmap. The wizard screen follows the same pattern: a `ratatui` component that collects the same inputs as the CLI wizard, then calls `assay_core::wizard::apply_gate_wizard`.

No new crate needed — new module in `assay-tui/src/screens/gate_wizard.rs` or similar.

---

## Component Boundary Diagram

```
assay-cli           assay-mcp           assay-tui
    |                   |                   |
    |  gate wizard cmd  |  gate_wizard tool |  wizard screen
    |                   |                   |
    +-------+-----------+-------------------+
            |
            v
    assay-core::wizard
        apply_gate_wizard()        (new)
        create_from_inputs()       (existing)
        create_spec_from_params()  (existing)
            |
            v
    assay-core::spec::compose
        resolve()                  (new)
        evaluate_preconditions()   (new)
            |
            +--> assay-core::criteria_library
                    load() / save() / scan()    (new)
                        |
                        v
                assay-types::CriteriaLibrary   (new)
                assay-types::SpecPreconditions (new)
                assay-types::GatesSpec         (modified: +3 fields)
```

---

## Modified Components

| Component | Change | Backward Compat |
|-----------|--------|-----------------|
| `assay-types::GatesSpec` | Add `extends: Option<String>`, `include: Vec<String>`, `preconditions: Option<SpecPreconditions>` | Yes — all optional with `#[serde(default)]` |
| `assay-types::GateSection` | No change | n/a |
| `assay-core::spec::validate` | Add validation for `extends` resolution, `include` resolution, precondition format | Additive — new diagnostics only |
| `assay-core::gate::mod` | `evaluate_all` (or new `evaluate_spec` wrapper) must first call `evaluate_preconditions`; skip criteria if preconditions fail | Behavior change — spec authors opt in via `[preconditions]` |
| `assay-core::wizard` | Add `apply_gate_wizard`, `PreconditionsInput`, `GateWizardInputs` | Additive |
| `assay-cli::commands::gate` | Add `wizard` subcommand | Additive |
| `assay-mcp::server` | Add 5 new tools | Additive |

---

## New Components (summary)

| Component | Location | Purpose |
|-----------|----------|---------|
| `CriteriaLibrary` | `assay-types/src/criteria_library.rs` | DTO for named criteria sets |
| `SpecPreconditions` | `assay-types/src/preconditions.rs` | DTO for precondition section |
| `PreconditionStatus` | `assay-types/src/preconditions.rs` | Evaluation result for preconditions |
| `spec::compose` | `assay-core/src/spec/compose.rs` | Resolution: extends + include merge, cycle detection |
| `criteria_library` | `assay-core/src/criteria_library.rs` | I/O for `.assay/criteria/*.toml` |
| `wizard::apply_gate_wizard` | `assay-core/src/wizard.rs` | Core wizard logic (update+create) |
| `commands::criteria` | `assay-cli/src/commands/criteria.rs` | `assay criteria list/new` |
| `GateWizard` screen | `assay-tui/src/screens/gate_wizard.rs` | TUI wizard component |

---

## Data Flow: Gate Run with Composability

```
gate run <spec>
    |
    v
spec::load(spec)                                        existing
    | raw GatesSpec (may have extends/include/preconditions)
    v
spec::compose::resolve(raw, load_library, load_parent)  NEW
    | ResolvedGatesSpec.effective_criteria
    v
gate::evaluate_preconditions(preconditions, ...)        NEW
    | PreconditionStatus
    | if !all_met -> return early with PreconditionFailed result
    v
gate::evaluate_all(effective_criteria, working_dir, ...)  existing (takes criteria slice)
    |
    v
history::save(GateRunRecord)                            existing
```

The key integration point: `gate::evaluate_all` already takes a `&[Criterion]` slice. Feeding it `ResolvedGatesSpec.effective_criteria` instead of `raw.criteria` is the only change to the evaluation callsite.

---

## Schema Registry

`CriteriaLibrary` and `SpecPreconditions` need `inventory::submit!` blocks for schema generation, consistent with all other types in `assay-types`. Add to `assay-types/src/lib.rs` re-exports.

---

## Suggested Build Order (dependency-first)

1. **`assay-types` additions** — `CriteriaLibrary`, `SpecPreconditions`, `PreconditionStatus`, `GatesSpec` field additions. Schema registration. No logic, no deps. Tests: TOML roundtrip, `skip_serializing_if`, `deny_unknown_fields` rejection.

2. **`assay-core::criteria_library`** — `load`/`save`/`scan` I/O. Depends on step 1. Follows `config::mod` and `milestone::mod` patterns exactly.

3. **`assay-core::spec::compose`** — `resolve()` with cycle detection. Depends on steps 1-2. Unit tests: single spec (noop), with `extends` (merge), with `include` (prepend), cycle detection, library-not-found error, parent-not-found error.

4. **`assay-core::spec::validate` update** — add diagnostics for unresolved `extends`, unknown `include` names, malformed `preconditions`. Depends on step 2.

5. **`assay-core::gate` integration** — wire `evaluate_preconditions` into `evaluate_all` or a new `evaluate_spec` wrapper. Depends on step 3.

6. **`assay-core::wizard` extension** — `apply_gate_wizard`. Depends on steps 1-3. Pure function, easy to unit test.

7. **`assay-cli` additions** — `gate wizard` subcommand + `criteria` subcommand. Depends on step 6. TTY-guarded.

8. **`assay-mcp` additions** — 5 new tools. Depends on step 6. JSON parameter validation.

9. **`assay-tui` wizard screen** — ratatui component. Depends on step 6. Can be built in parallel with steps 7-8 after step 6 is done.

---

## Critical Integration Points

**`GatesSpec` deny_unknown_fields constraint**: The struct uses `#[serde(deny_unknown_fields)]`. Adding new optional fields is safe as long as all three new fields have `#[serde(default)]` and `#[serde(skip_serializing_if)]`. Without `skip_serializing_if`, roundtripped files would gain new empty fields, breaking content-equality assertions in tests.

**`spec::compose::resolve` must be lazy**: Libraries and parent specs are loaded on demand inside `resolve()` via closure parameters. This allows callers (tests, CLI, MCP) to supply different loading strategies (real disk vs in-memory fixtures) without coupling the resolver to file I/O — consistent with the closure-based control inversion convention throughout assay-core.

**Precondition evaluation requires gate history**: `evaluate_preconditions` checks whether `requires` slugs have passing gate runs. This is a read-only query against history. If integrating this check inside `gate::mod` would create a circular dependency with `history::mod`, pull the precondition check to the callsite (CLI/MCP handler) instead of inside `gate::mod`. The data dependency is: history result feeds into whether to run gates, so the callsite-level check is architecturally cleaner.

**Wizard update mode vs create mode**: `apply_gate_wizard` must handle both new and existing specs. For existing specs, it loads the current `GatesSpec`, merges composability fields in, then atomically writes back. The merge strategy for `criteria`: composability fields replace wholesale; `criteria` is appended or replaced based on a `replace_criteria: bool` parameter in `GateWizardInputs`.

**MCP `spec_resolve` output**: Agents calling `spec_resolve` need the full effective criteria list. The response should annotate each criterion with its source (`library:<name>`, `parent:<slug>`, or `own`) to aid debugging inheritance chains. This annotation is computed in the response serializer, not stored in the type.

---

## Patterns to Follow

### Atomic writes
Library and spec writes must use `NamedTempFile` + `persist`. See `wizard::write_gates_toml` for the canonical pattern. Do not write directly to the target path.

### Free functions, no traits (zero-trait convention)
`spec::compose::resolve` takes closure parameters for loading dependencies — not a `SpecLoader` trait. This is the control-inversion pattern used throughout (e.g., pipeline closures in assay-core).

### `deny_unknown_fields` on new DTOs
`CriteriaLibrary` and `SpecPreconditions` should have `#[serde(deny_unknown_fields)]` to surface typos early. This matches the existing convention on `GatesSpec`, `GateSection`, and `Criterion`.

### Additive MCP tools
The established decision: never change parameter signatures of existing MCP tools. All composability tools are new additions alongside existing ones.

---

## Gaps / Open Questions

1. **Criteria merge conflict resolution**: If both parent spec and own spec define a criterion with the same `name`, should own override silently or error? The merge order (library then parent then own) implies own wins, but this needs an explicit decision to avoid silent shadowing bugs.

2. **`preconditions.requires` temporal semantics**: "Spec X has passing gates" — passing at what time? Last run? Last run within N days? The simplest implementation is "last recorded gate run passed". Staleness handling can be deferred to a follow-on.

3. **Circular library includes**: Libraries including other libraries is not in scope for v0.7.0 (libraries are flat). If needed later, the same cycle-detection approach from `spec::compose::resolve` applies.

4. **TUI wizard complexity vs priority**: The TUI wizard is the most implementation-heavy piece. If the milestone requires shipping CLI + MCP first and TUI later, the build order supports that split cleanly — step 9 is independent of steps 7 and 8.
