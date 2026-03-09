# Phase 27: Types Hygiene — Research

## Standard Stack

No new dependencies. All work uses:

- **Hand-written `impl Display`** for enum Display (zero new deps; strum is excluded by hard constraint)
- **`#![deny(missing_docs)]`** crate-level attribute in `assay-types` (locked decision)
- **`#[deny(clippy::derive_partial_eq_without_eq)]`** crate-level attribute (locked decision)
- **Workspace `[workspace.lints.clippy]`** in root `Cargo.toml` for the clippy deny (preferred over per-crate attributes for workspace consistency)

## Architecture Patterns

### Eq Derive Pattern

Types with `PartialEq` but without `Eq` that **can safely add `Eq`** (no float fields, no transitive float dependencies):

| File | Type | Currently | Blocker? |
|------|------|-----------|----------|
| `lib.rs` | `Spec` | `PartialEq` | None — all fields are String/Vec/Option of Eq types |
| `enforcement.rs` | `Enforcement` | `PartialEq` | None — Copy enum, no data |
| `enforcement.rs` | `GateSection` | `PartialEq` | None — contains `Enforcement` (Eq-safe) |
| `enforcement.rs` | `EnforcementSummary` | `PartialEq` | None — all `usize` fields |
| `criterion.rs` | `Criterion` | `PartialEq` | None — String/Option<String>/Option<Enforcement>/Option<CriterionKind> |
| `gate.rs` | `GateKind` | `PartialEq` | None — String data variants only |
| `gate_run.rs` | `GateRunSummary` | `PartialEq` | None — all Eq-safe after GateResult/CriterionResult get Eq |
| `gate_run.rs` | `CriterionResult` | `PartialEq` | None — after GateResult gets Eq |
| `gate_run.rs` | `GateRunRecord` | `PartialEq` | None — after GateRunSummary gets Eq, DateTime<Utc> is Eq |
| `gates_spec.rs` | `GateCriterion` | `PartialEq` | None — same fields as Criterion + Vec<String> |
| `gates_spec.rs` | `GatesSpec` | `PartialEq` | None — after GateCriterion gets Eq |
| `session.rs` | `AgentEvaluation` | `PartialEq` | None — DateTime<Utc> is Eq, all other fields Eq-safe |
| `session.rs` | `AgentSession` | `PartialEq` | None — HashMap/HashSet of Eq types |
| `feature_spec.rs` | ALL types (15 types) | `PartialEq` | None — all String/Vec/enum fields |

Types that **cannot derive `Eq`** (contain `f64`):

| File | Type | Float Field |
|------|------|-------------|
| `lib.rs` | `GuardConfig` | `soft_threshold: f64`, `hard_threshold: f64` |
| `checkpoint.rs` | `ContextHealthSnapshot` | `utilization_pct: f64` |
| `context.rs` | `BloatEntry` | `percentage: f64` |
| `context.rs` | `DiagnosticsReport` | `context_utilization_pct: Option<f64>` |
| `context.rs` | `TokenEstimate` | `context_utilization_pct: f64` |

**GateResult** (`gate.rs:46`) is the key dependency — it has `PartialEq` without `Eq` but contains NO float fields. It contains `DateTime<Utc>` (Eq), `GateKind` (Eq-safe), `Option<Confidence>` (already Eq), `Option<EvaluatorRole>` (already Eq). Adding `Eq` to `GateResult` unlocks `CriterionResult`, `GateRunSummary`, `GateRunRecord`.

**Confidence**: HIGH. All assessments verified by field inspection.

### Display Impl Pattern

Enums that need `Display` (user-facing, used in CLI/MCP output):

| Enum | Variants | Recommended Display Format |
|------|----------|---------------------------|
| `Enforcement` | `Required`, `Advisory` | Lowercase: `"required"`, `"advisory"` (matches serde kebab-case) |
| `GateKind` | `Command{cmd}`, `AlwaysPass`, `FileExists{path}`, `AgentReport` | Variant name only: `"Command"`, `"AlwaysPass"`, `"FileExists"`, `"AgentReport"` |
| `ContextHealth` | `Healthy`, `Warning`, `Critical` | Lowercase: `"healthy"`, `"warning"`, `"critical"` |
| `BloatCategory` | 6 variants | Already has `label()` method; `Display` should delegate to `label()` |
| `PruneStrategy` | 6 variants | Already has `label()` method; `Display` should delegate to `label()` |
| `PrescriptionTier` | `Gentle`, `Standard`, `Aggressive` | Lowercase: `"gentle"`, `"standard"`, `"aggressive"` |
| `SpecStatus` | 6 variants | Kebab-case: `"draft"`, `"in-progress"`, etc. (match serde) |
| `Obligation` | `Shall`, `Should`, `May` | Lowercase: `"shall"`, `"should"`, `"may"` |
| `Priority` | `Must`, `Should`, `Could`, `Wont` | Lowercase: `"must"`, `"should"`, `"could"`, `"wont"` |
| `VerificationMethod` | `Test`, `Analysis`, `Inspection`, `Demonstration` | Lowercase: match serde |
| `AcceptanceCriterionType` | `Gherkin`, `Ears`, `Plain` | Lowercase: match serde |
| `ImpactLevel` | `Low`, `Medium`, `High`, `Critical` | Lowercase: match serde |
| `LikelihoodLevel` | `Low`, `Medium`, `High` | Lowercase: match serde |
| `Confidence` | `High`, `Medium`, `Low` | Lowercase: match serde |
| `EvaluatorRole` | `SelfEval`, `Independent`, `Human` | Serde form: `"self"`, `"independent"`, `"human"` |
| `AgentStatus` | `Active`, `Idle`, `Done`, `Unknown` | Snake_case: `"active"`, `"idle"`, `"done"`, `"unknown"` |
| `TaskStatus` | `Pending`, `InProgress`, `Completed`, `Cancelled` | Snake_case: `"pending"`, `"in_progress"`, `"completed"`, `"cancelled"` |
| `CriterionKind` | `AgentReport` | `"AgentReport"` (PascalCase, matches serde) |

**Recommendation**: Display output should match the serde serialization form for consistency. This means kebab-case for most feature_spec enums, snake_case for checkpoint enums, and PascalCase for GateKind/CriterionKind. For `GateKind` data variants (`Command`, `FileExists`), display only the variant name without the data — the data is available via the struct fields.

For `BloatCategory` and `PruneStrategy` which already have `label()` methods, `Display` should delegate to `label()` for human-readable output.

**Confidence**: HIGH.

### GateSection Default

`GateSection` currently has one field:
```rust
pub struct GateSection {
    pub enforcement: Enforcement,  // Enforcement already has #[default] = Required
}
```

Deriving `Default` is trivial — `Enforcement` already derives `Default` with `Required` as the default variant.

**Confidence**: HIGH.

### Criterion Dedup Strategy

**Current state:**
- `Criterion` (criterion.rs): 8 fields — name, description, cmd, path, timeout, enforcement, kind, prompt
- `GateCriterion` (gates_spec.rs): 9 fields — same 8 + `requirements: Vec<String>`

The only difference is the `requirements` field. The `to_criterion()` function in `gate/mod.rs` does a field-by-field clone, dropping `requirements`.

**Recommended approach**: Merge into a single `Criterion` type with an added `requirements` field:
```rust
/// Requirement IDs this criterion traces to (e.g., `["REQ-FUNC-001"]`).
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub requirements: Vec<String>,
```

This is serde-compatible: existing flat-file specs (which lack `requirements`) will deserialize correctly because `Vec::is_empty` + `default` means the field is optional. The `GateCriterion` type becomes a type alias (`pub type GateCriterion = Criterion`) for backward compatibility, or is removed entirely with callers updated to use `Criterion`.

**CriterionResult review** (locked requirement): The name `CriterionResult` is clear and consistent. The type has `criterion_name: String` — this could be renamed to just `name` for brevity but that reduces clarity. No structural changes needed; the naming is adequate.

**Confidence**: HIGH for the merge approach. The `requirements` field with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` is backward-compatible.

### Doc Comments Audit

47 warnings from `RUSTDOCFLAGS="-W missing_docs" cargo doc --no-deps -p assay-types`:

| File | Count | What's Missing |
|------|-------|----------------|
| `lib.rs` | 9 | Crate-level doc, `Gate` fields (name, passed), `Review` fields, `Workflow` fields |
| `context.rs` | 7 | `ContentBlock` variant fields (text, thinking, tool_use fields, tool_result fields) |
| `enforcement.rs` | 4 | `EnforcementSummary` fields (required_passed, required_failed, advisory_passed, advisory_failed) |
| `feature_spec.rs` | 27 | Enum variants on SpecStatus, Obligation, Priority, VerificationMethod, AcceptanceCriterionType, ImpactLevel, LikelihoodLevel |

All other modules already have complete doc comments.

**Confidence**: HIGH. Exact count and locations verified via `cargo doc`.

## Don't Hand-Roll

- **Do NOT use strum or any derive macro for Display** — zero new workspace deps constraint
- **Do NOT implement `Eq` on types with `f64` fields** — `GuardConfig`, `ContextHealthSnapshot`, `BloatEntry`, `DiagnosticsReport`, `TokenEstimate`
- **Do NOT change serde serialization format** — Display output matches serde form but does not alter `#[serde(rename_all)]` attributes
- **Do NOT add `Hash` alongside `Eq`** — only add `Hash` where it's actually needed (most types don't need it)

## Common Pitfalls

1. **Eq on types containing GateResult transitively** — GateResult must get Eq FIRST, then CriterionResult, then GateRunSummary, then GateRunRecord. Order matters for compilation.

2. **GateCriterion removal breaks downstream** — `GateCriterion` is used extensively in `assay-core/src/gate/mod.rs` and `assay-core/src/spec/mod.rs`. If removing the type (not aliasing), every usage site must be updated. A type alias is the safest migration path.

3. **`#![deny(missing_docs)]` breaks build immediately** — Must add ALL doc comments in the same commit/task that adds the deny attribute. Adding the deny first will break CI.

4. **`deny(clippy::derive_partial_eq_without_eq)` triggers on float types** — Types with `f64` fields that derive `PartialEq` but not `Eq` will trigger this lint. Must use `#[allow(clippy::derive_partial_eq_without_eq)]` on those specific types, or place the deny at the workspace lint level and allow per-type.

5. **Criterion merge changes schema output** — Adding `requirements` to `Criterion` changes its JSON Schema. The `inventory::submit!` for both `"criterion"` and `"gate-criterion"` should be updated. Schema snapshots (if tested with insta) will need updating.

6. **`to_criterion()` becomes trivial after merge** — If `GateCriterion` becomes a type alias for `Criterion`, the `to_criterion()` function in `gate/mod.rs` becomes `fn to_criterion(gc: &Criterion) -> Criterion { gc.clone() }` or can be removed entirely.

## Code Examples

### Hand-written Display impl (matching serde form)

```rust
impl std::fmt::Display for Enforcement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Required => write!(f, "required"),
            Self::Advisory => write!(f, "advisory"),
        }
    }
}
```

### Display delegating to existing label() method

```rust
impl std::fmt::Display for BloatCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}
```

### Display for GateKind (variant name only, no data)

```rust
impl std::fmt::Display for GateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command { .. } => write!(f, "Command"),
            Self::AlwaysPass => write!(f, "AlwaysPass"),
            Self::FileExists { .. } => write!(f, "FileExists"),
            Self::AgentReport => write!(f, "AgentReport"),
        }
    }
}
```

### Criterion merge with backward-compatible requirements field

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub enforcement: Option<Enforcement>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kind: Option<CriterionKind>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub prompt: Option<String>,
    /// Requirement IDs this criterion traces to (e.g., `["REQ-FUNC-001"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<String>,
}
```

### Per-type allow for float types that can't derive Eq

```rust
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GuardConfig { /* ... f64 fields ... */ }
```

### Workspace-level clippy lint configuration

```toml
# In root Cargo.toml
[workspace.lints.clippy]
derive_partial_eq_without_eq = "deny"
```

Then in each crate's `Cargo.toml`:
```toml
[lints]
workspace = true
```

## Open Questions

1. **Type alias vs full removal for GateCriterion**: A type alias (`pub type GateCriterion = Criterion`) preserves all downstream usage sites unchanged. Full removal is cleaner but requires updating ~30 usage sites across `assay-core`. Recommendation: type alias for this phase, full removal in a future cleanup.

2. **`to_criterion()` removal**: After the merge, `to_criterion()` becomes a clone. It could be kept as documentation of intent ("we only use the evaluation-relevant fields") or removed. Recommendation: replace with `.clone()` at call sites if the alias approach is used.

3. **Schema registry entries**: After merge, both `"criterion"` and `"gate-criterion"` schemas will be identical. Keep both registry entries for backward compatibility of schema consumers, or remove `"gate-criterion"`? Recommendation: keep both, with `"gate-criterion"` generating the same schema.
