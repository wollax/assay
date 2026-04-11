# Phase 64: Type Foundation - Research

**Researched:** 2026-04-11
**Domain:** Rust type design — serde, schemars, insta, TOML backward compatibility
**Confidence:** HIGH

## Summary

Phase 64 is a pure `assay-types` extension. No new crates beyond `semver` are needed. The existing patterns in the codebase are completely consistent and well-established: all optional fields use `#[serde(default, skip_serializing_if)]`, all user-facing TOML structs use `#[serde(deny_unknown_fields)]`, new types register with `inventory::submit!`, and schema changes are captured with `insta::assert_json_snapshot!`.

The critical integration challenge is `CriteriaLibrary.version`: the CONTEXT.md decision says "parse as `semver::Version`", but `semver::Version` does not implement `JsonSchema`. The clean resolution is to store version as a newtype wrapper (`SemverString`) that serializes/deserializes as a `String` and implements `JsonSchema` as `type: "string"`, with validation happening at parse time via `semver::Version::parse()`. This keeps the struct `JsonSchema`-derivable without a custom schema implementation.

Backward-compatibility for SAFE-03 is guaranteed by `#[serde(default)]` on all three new `GatesSpec` fields (`extends`, `include`, `preconditions`). Pre-v0.7.0 TOML files without these fields will continue to parse cleanly because `deny_unknown_fields` rejects unknowns (which is correct) and `default` fills in absent fields.

**Primary recommendation:** Add the three fields to `GatesSpec`, implement `CriteriaLibrary`/`SpecPreconditions`/`PreconditionStatus` in separate source files following existing module conventions, add `semver` as a workspace direct dependency, then update snapshots with `cargo insta review`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**CriteriaLibrary shape:**
- Rich metadata: name (slug), description, version, tags, and criteria list
- Version field is semver-validated (use `semver` crate, parse as `semver::Version`)
- description, version, and tags are all optional with serde defaults — minimal valid library is just name + criteria
- Uses `#[serde(deny_unknown_fields)]` — consistent with GatesSpec and Criterion

**Precondition modeling:**
- Preconditions live on the gate (`gates.toml`), not on the spec
- `commands` is `Vec<String>` — simple shell command strings, no structured command objects
- `requires` references spec slugs — "did this spec's last gate run pass?"
- Uses `#[serde(deny_unknown_fields)]`

**Include behavior:**
- `include` is `Vec<String>` — multiple libraries can be included
- Library criteria merge flat into the gate's criteria list (no namespacing)
- Conflict resolution order: own > last-listed library > first-listed library
- When both `extends` and `include` are present: library criteria (base) → parent criteria (override libs) → own criteria (override everything)

**PreconditionStatus design:**
- Per-condition breakdown: each `requires` entry and each `command` gets its own pass/fail status
- `RequireStatus`: spec_slug + passed bool
- `CommandStatus`: command string + passed bool + optional combined output (stdout+stderr merged)
- Evaluate all conditions (no short-circuit)

### Claude's Discretion
- Whether precondition command output uses existing head+tail truncation or simpler handling
- Exact serde field ordering and skip_serializing_if patterns (follow existing conventions)
- Schema snapshot update mechanics

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INHR-01 | User can define a gate that extends another gate via `gate.extends` field | GatesSpec gains `extends: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` |
| INHR-02 | Extended gate inherits parent criteria with own-wins merge semantics | `GatesSpec.include: Vec<String>` for library includes; merge semantics documented — types only, no runtime logic in this phase |
| SAFE-03 | All new GatesSpec fields are backward-compatible (existing TOML files parse without error) | `#[serde(default)]` on all three new fields guarantees pre-v0.7.0 TOML files parse cleanly |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde | 1.x (workspace) | Serialize/Deserialize derives | All types in assay-types use it |
| schemars | 1.x (workspace) | JsonSchema derive | All public types implement it for schema generation |
| inventory | 0.3 (workspace) | Schema auto-registration | Established pattern: `inventory::submit!` at definition site |
| insta | 1.46 (workspace) | Snapshot tests | All schema snapshot tests use it |
| toml | 1.x (dev, workspace) | TOML roundtrip tests | All gate spec tests serialize/deserialize via toml crate |
| semver | 1.0.x (new direct dep) | Semver version validation for CriteriaLibrary.version | Required by CONTEXT.md decision |

### Important: semver and JsonSchema
`semver::Version` does NOT implement `JsonSchema` (confirmed from docs.rs). The resolved approach:
- Store version as a `String` in the struct (for JsonSchema compatibility)
- Validate via `semver::Version::parse()` in a custom `Deserialize` impl or a newtype wrapper
- Schema type for the version field: `"string"` with a description noting semver format

The simplest approach matching existing conventions: use `Option<String>` and validate lazily, or use a newtype `SemverVersion(String)` with custom Deserialize that calls `semver::Version::parse()`.

**Recommendation:** Use `Option<String>` for the field type (schema stays simple), add `semver` to workspace deps for validation elsewhere in core. This keeps `CriteriaLibrary` fully `JsonSchema`-derivable without custom implementations. `semver` validation belongs in `assay-core` not `assay-types` (types crate has no business logic per CLAUDE.md).

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| jsonschema | 0.43 (dev) | Schema validation in roundtrip tests | Add roundtrip validation tests for new types |

### Installation
```bash
# Add semver to workspace Cargo.toml [workspace.dependencies]:
semver = { version = "1", features = ["serde"] }

# Add to crates/assay-types/Cargo.toml [dependencies]:
semver.workspace = true
```

Note: `semver` 1.0.27 is already in Cargo.lock as a transitive dependency. Adding it as a direct workspace dependency makes the version intent explicit and allows feature control.

## Architecture Patterns

### Recommended Project Structure
```
crates/assay-types/src/
├── gates_spec.rs         # Add extends, include, preconditions fields to GatesSpec
├── criteria_library.rs   # NEW: CriteriaLibrary struct
├── precondition.rs       # NEW: SpecPreconditions, PreconditionStatus, RequireStatus, CommandStatus
└── lib.rs                # Add pub mod + re-exports for new types
```

### Pattern 1: Optional Field with Backward Compat
**What:** All new optional fields on existing structs get `#[serde(default, skip_serializing_if)]`.
**When to use:** Every new optional field added to a user-facing TOML type.
**Example (from gates_spec.rs):**
```rust
// Existing pattern for reference:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub milestone: Option<String>,

#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub depends: Vec<String>,

// New fields follow the same pattern:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub extends: Option<String>,

#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub include: Vec<String>,

#[serde(default, skip_serializing_if = "Option::is_none")]
pub preconditions: Option<SpecPreconditions>,
```

### Pattern 2: New Type Module with Schema Registration
**What:** Each new type in its own source file with `inventory::submit!` at the bottom.
**When to use:** All new public types that appear in JSON schemas.
**Example (from criterion.rs):**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriteriaLibrary {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,  // semver-format string; validated in assay-core
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub criteria: Vec<crate::Criterion>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "criteria-library",
        generate: || schemars::schema_for!(CriteriaLibrary),
    }
}
```

### Pattern 3: Schema Snapshot Update Workflow
**What:** After adding new types and modifying existing ones, update snapshots with `cargo insta`.
**When to use:** Any time schema changes.
**Procedure:**
1. Add snapshot tests for new types in `crates/assay-types/tests/schema_snapshots.rs`
2. Run `cargo test -p assay-types` — tests fail with "missing snapshot"
3. Run `cargo insta review` to accept new/changed snapshots
4. Commit the `.snap` files

### Pattern 4: Roundtrip + Backward Compat Tests
**What:** Every new/modified type gets TOML roundtrip AND legacy-TOML backward compat tests.
**When to use:** All user-facing TOML structs.
**What to test:**
- New type with all fields populated: TOML serialize → deserialize → assert_eq
- New type with only required fields: minimal TOML parses without error
- For `GatesSpec`: pre-v0.7.0 TOML (no new fields) still parses cleanly
- For `GatesSpec`: new fields are omitted from serialization when default/empty
- `deny_unknown_fields`: TOML with spurious field produces a parse error

### Anti-Patterns to Avoid
- **Missing `#[serde(default)]` on new optional fields:** TOML files without the field fail to parse — breaks SAFE-03.
- **Putting `semver::Version` directly in a struct:** It doesn't implement `JsonSchema`, causing compile errors on `#[derive(JsonSchema)]`.
- **Business logic in assay-types:** Types crate has no business logic (CLAUDE.md); merge semantics, validation, and semver parsing go in `assay-core`.
- **Forgetting `deny_unknown_fields` on new types:** CONTEXT.md explicitly requires it for all new types.
- **Missing `inventory::submit!` block:** New types won't appear in generated schemas.
- **Forgetting `pub use` in lib.rs:** New types won't be part of the public API.
- **Forgetting to update snapshot for modified GatesSpec:** The `gates-spec-schema` snapshot test will fail with a diff mismatch if not regenerated.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Semver parsing | Custom version string validator | `semver::Version::parse()` | Handles pre-release, build metadata, all edge cases |
| Schema generation | Manual schema JSON | `schemars::schema_for!()` | Derives from type definitions, stays in sync |
| Snapshot management | Manual .snap file editing | `cargo insta review` | Insta's review workflow prevents accidental drift |
| Inventory collection | Custom type registry | `inventory::submit!` / `inventory::iter` | Already established in schema_registry.rs |

## Common Pitfalls

### Pitfall 1: `deny_unknown_fields` + `default` interaction
**What goes wrong:** Developer adds `deny_unknown_fields` to a struct but forgets `default` on optional fields — TOML without those fields fails with "missing field" error instead of using defaults.
**Why it happens:** `deny_unknown_fields` and missing-field errors are separate mechanisms. `deny_unknown_fields` rejects extra fields; `#[serde(default)]` fills in missing ones. Both must be present.
**How to avoid:** Every optional field on a `deny_unknown_fields` struct gets `#[serde(default)]`.
**Warning signs:** Backward compat test ("legacy TOML parses cleanly") fails with "missing field: `extends`" or similar.

### Pitfall 2: `semver::Version` in a JsonSchema-derived struct
**What goes wrong:** `#[derive(JsonSchema)]` fails to compile because `semver::Version` does not implement `JsonSchema`.
**Why it happens:** The `semver` crate does not have an optional `schemars` feature.
**How to avoid:** Store version as `Option<String>` in the type; validate semver format in `assay-core` (not in `assay-types`).
**Warning signs:** Compile error: "the trait `JsonSchema` is not implemented for `semver::Version`".

### Pitfall 3: Duplicate snapshot test name
**What goes wrong:** Two snapshot tests assert against the same snapshot name — last one to run wins, masking failures.
**Why it happens:** The existing `schema_snapshots.rs` already has both `gates_spec_schema_snapshot` (line 69) and `gates_spec_schema_updated_snapshot` (line 254) asserting against `"gates-spec-schema"`. This is a pre-existing issue.
**How to avoid:** When adding new snapshot tests for new types, use unique names. Do not add another duplicate for `"gates-spec-schema"` — the existing test at line 69 (`gates_spec_schema_snapshot`) is authoritative.
**Warning signs:** `cargo test` passes even though schema changed, because the second test overwrites the first's assertion.

### Pitfall 4: Missing `pub use` in lib.rs
**What goes wrong:** New types compile but are not re-exported, so `assay-core` consumers can't use them without `assay_types::criteria_library::CriteriaLibrary` paths.
**Why it happens:** All existing public types are re-exported at the crate root in `lib.rs`.
**How to avoid:** Add `pub mod criteria_library;` and `pub use criteria_library::CriteriaLibrary;` etc. to `lib.rs`.

### Pitfall 5: `#[deny(missing_docs)]` — lib.rs has it
**What goes wrong:** New public types/fields without doc comments cause compile errors.
**Why it happens:** `lib.rs` has `#![deny(missing_docs)]` at line 8.
**How to avoid:** Every new `pub struct`, `pub field`, and `pub enum variant` needs a `///` doc comment.

## Code Examples

### GatesSpec with new fields
```rust
// crates/assay-types/src/gates_spec.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GatesSpec {
    // ... existing fields ...

    /// Slug of the parent gate spec this one extends. When set, the parent's
    /// criteria are inherited with own-wins merge semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// Criteria library slugs to include. Criteria from each library are
    /// merged flat into this gate's criteria list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,

    /// Preconditions that must be met before gate evaluation proceeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<SpecPreconditions>,

    /// Gate criteria that must be satisfied.
    pub criteria: Vec<GateCriterion>,
}
```

### SpecPreconditions struct
```rust
// crates/assay-types/src/precondition.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpecPreconditions {
    /// Spec slugs whose last gate run must have passed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,

    /// Shell commands that must exit 0 before gate evaluation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
}
```

### PreconditionStatus and sub-types
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PreconditionStatus {
    /// Per-spec-require results.
    pub requires: Vec<RequireStatus>,
    /// Per-command results.
    pub commands: Vec<CommandStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RequireStatus {
    /// The spec slug that was checked.
    pub spec_slug: String,
    /// Whether the spec's last gate run passed.
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommandStatus {
    /// The shell command that was evaluated.
    pub command: String,
    /// Whether the command exited 0.
    pub passed: bool,
    /// Combined stdout+stderr output. Omitted when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}
```

### Backward compat test pattern (from gates_spec.rs)
```rust
#[test]
fn gates_spec_legacy_toml_without_composability_fields_parses_cleanly() {
    let toml_str = r#"
name = "legacy-spec"

[[criteria]]
name = "check"
description = "A check"
"#;
    let spec: GatesSpec = toml::from_str(toml_str)
        .expect("legacy TOML without new fields must parse cleanly");
    assert!(spec.extends.is_none());
    assert!(spec.include.is_empty());
    assert!(spec.preconditions.is_none());
}

#[test]
fn gates_spec_with_extends_and_include_roundtrip() {
    let toml_str = r#"
name = "child-spec"
extends = "parent-gate"
include = ["lib-a", "lib-b"]

[[criteria]]
name = "check"
description = "A check"
"#;
    let spec: GatesSpec = toml::from_str(toml_str).expect("parse with extends+include");
    assert_eq!(spec.extends, Some("parent-gate".to_string()));
    assert_eq!(spec.include, vec!["lib-a", "lib-b"]);

    let re_serialized = toml::to_string(&spec).expect("re-serialize");
    let roundtripped: GatesSpec = toml::from_str(&re_serialized).expect("roundtrip");
    assert_eq!(spec, roundtripped);
}
```

### Snapshot test additions for schema_snapshots.rs
```rust
#[test]
fn criteria_library_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::CriteriaLibrary);
    assert_json_snapshot!("criteria-library-schema", schema.to_value());
}

#[test]
fn spec_preconditions_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::SpecPreconditions);
    assert_json_snapshot!("spec-preconditions-schema", schema.to_value());
}

#[test]
fn precondition_status_schema_snapshot() {
    let schema = schemars::schema_for!(assay_types::PreconditionStatus);
    assert_json_snapshot!("precondition-status-schema", schema.to_value());
}
```

## State of the Art

| Old Approach | Current Approach | Notes |
|--------------|------------------|-------|
| Separate GateCriterion struct | Type alias `GateCriterion = Criterion` | Already done — criteria_library.rs reuses `Criterion` directly |
| Manual schema JSON files | `schemars::schema_for!()` + `inventory::submit!` | Auto-registration pattern already established |

## Open Questions

1. **`semver` in assay-types vs. string storage**
   - What we know: `semver::Version` does not implement `JsonSchema`; CONTEXT.md says "parse as `semver::Version`"; types crate has no business logic
   - What's unclear: Whether validation should happen at deserialization in types (via custom Deserialize) or deferred to core
   - Recommendation: Store `version` as `Option<String>` in `CriteriaLibrary` (types crate stays schema-derivable and has no logic); add `semver` to workspace deps but use it only in `assay-core` for validation. The planner should treat the `semver` workspace dep addition as a types-phase task even though validation lands in core.

2. **`PreconditionStatus` schema registration**
   - What we know: Status types (`PreconditionStatus`, `RequireStatus`, `CommandStatus`) are runtime output, not user-authored TOML
   - What's unclear: Whether they need `inventory::submit!` entries in this phase (schema gen) or just the TOML-authored types
   - Recommendation: Register all new public types with `inventory::submit!` for consistency; the schema generator emits all registered types regardless of whether they appear in gates.toml.

3. **Ordering of fields in GatesSpec**
   - What we know: Serde field order in structs determines TOML serialization order; existing fields have a defined order
   - What's unclear: Where exactly `extends`, `include`, `preconditions` should appear relative to `criteria`
   - Recommendation: Place `extends` and `include` before `criteria` (they conceptually compose the criteria list); place `preconditions` before `criteria` as well (it gates evaluation). Final order: name, description, gate, depends, milestone, order, **extends, include, preconditions**, criteria.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test + insta 1.46 |
| Config file | `crates/assay-types/Cargo.toml` (dev-deps: insta, toml, jsonschema) |
| Quick run command | `cargo test -p assay-types` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INHR-01 | `extends = "parent-gate"` deserializes into GatesSpec | unit (TOML roundtrip) | `cargo test -p assay-types gates_spec_with_extends` | ❌ Wave 0 |
| INHR-02 | `include = ["lib-name"]` deserializes into GatesSpec | unit (TOML roundtrip) | `cargo test -p assay-types gates_spec_with_include` | ❌ Wave 0 |
| SAFE-03 | Pre-v0.7.0 TOML (no new fields) parses without error | unit (backward compat) | `cargo test -p assay-types legacy_toml_without_composability` | ❌ Wave 0 |
| SAFE-03 | JSON schema snapshots include new fields, no drift | snapshot | `cargo test -p assay-types schema_snapshots` | ❌ Wave 0 (snapshots need regeneration) |

### Sampling Rate
- **Per task commit:** `cargo test -p assay-types`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/kata:verify-work`

### Wave 0 Gaps
- [ ] New test functions in `crates/assay-types/src/gates_spec.rs` — covers INHR-01, INHR-02, SAFE-03 (backward compat)
- [ ] New test functions in `crates/assay-types/src/criteria_library.rs` — covers CriteriaLibrary roundtrip
- [ ] New test functions in `crates/assay-types/tests/schema_snapshots.rs` — snapshot tests for new types
- [ ] Run `cargo insta review` after implementing — regenerates `gates-spec-schema` snapshot and creates new snapshots
- [ ] Schema roundtrip tests in `crates/assay-types/tests/schema_roundtrip.rs` — validate new types against their schemas

## Sources

### Primary (HIGH confidence)
- Codebase direct read: `crates/assay-types/src/gates_spec.rs`, `criterion.rs`, `lib.rs`, `schema_registry.rs` — established patterns confirmed
- Codebase direct read: `crates/assay-types/tests/schema_snapshots.rs`, `schema_roundtrip.rs` — test patterns confirmed
- `Cargo.lock` — semver 1.0.27 confirmed as transitive dependency with serde feature
- `docs.rs/semver/1.0.27/semver/struct.Version.html` — confirmed `JsonSchema` not implemented

### Secondary (MEDIUM confidence)
- docs.rs semver crate — no `schemars` optional feature listed (no `jsonschema` feature either)

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies confirmed from workspace Cargo.toml and Cargo.lock
- Architecture: HIGH — patterns confirmed by reading actual source files
- Pitfalls: HIGH — `deny_unknown_fields`+`default` interaction verified from test code; `semver::Version` JsonSchema gap verified from docs.rs; `#![deny(missing_docs)]` verified from lib.rs line 8; duplicate snapshot test confirmed from schema_snapshots.rs lines 69 and 254

**Research date:** 2026-04-11
**Valid until:** 2026-05-11 (stable crate versions, no fast-moving parts)
