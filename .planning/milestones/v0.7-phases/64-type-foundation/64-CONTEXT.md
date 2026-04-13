# Phase 64: Type Foundation - Context

**Gathered:** 2026-04-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Add composability primitives to `assay-types`: `CriteriaLibrary`, `SpecPreconditions`, `PreconditionStatus` (with sub-types), and three additive fields on `GatesSpec` (`extends`, `include`, `preconditions`). Update JSON schema snapshots. All existing TOML files must continue to parse without error.

</domain>

<decisions>
## Implementation Decisions

### CriteriaLibrary shape
- Rich metadata: name (slug), description, version, tags, and criteria list
- Version field is semver-validated (use `semver` crate, parse as `semver::Version`)
- description, version, and tags are all optional with serde defaults — minimal valid library is just name + criteria
- Uses `#[serde(deny_unknown_fields)]` — consistent with GatesSpec and Criterion

### Precondition modeling
- Preconditions live on the gate (`gates.toml`), not on the spec
- `commands` is `Vec<String>` — simple shell command strings, no structured command objects
- `requires` references spec slugs (not gate-qualified names) — "did this spec's last gate run pass?"
- Uses `#[serde(deny_unknown_fields)]`

### Include behavior
- `include` is `Vec<String>` — multiple libraries can be included
- Library criteria merge flat into the gate's criteria list (no namespacing)
- Conflict resolution order: own > last-listed library > first-listed library
- When both `extends` and `include` are present, resolution order: library criteria (base) → parent criteria (override libs) → own criteria (override everything)

### PreconditionStatus design
- Per-condition breakdown: each `requires` entry and each `command` gets its own pass/fail status
- `RequireStatus`: spec_slug + passed bool
- `CommandStatus`: command string + passed bool + optional combined output (stdout+stderr merged)
- Evaluate all conditions (no short-circuit) — user sees full picture in one gate run

### Claude's Discretion
- Whether precondition command output uses existing head+tail truncation or simpler handling
- Exact serde field ordering and skip_serializing_if patterns (follow existing conventions)
- Schema snapshot update mechanics

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `GatesSpec` (`crates/assay-types/src/gates_spec.rs`): Target struct for new `extends`, `include`, `preconditions` fields. Currently uses `#[serde(deny_unknown_fields)]`.
- `Criterion` (`crates/assay-types/src/criterion.rs`): Reuse as `CriteriaLibrary.criteria` element type. Already has all needed derives and serde attributes.
- `GateCriterion` type alias: Points to `Criterion`, used by `GatesSpec.criteria`.
- Schema registry (`inventory::submit!` pattern): Used for JSON schema generation. New types need registry entries.

### Established Patterns
- All optional fields use `#[serde(default, skip_serializing_if = "...")]` — backward compat mandatory
- `#[serde(deny_unknown_fields)]` on all user-facing TOML types
- `inventory::submit!` for schema registration
- Roundtrip tests (TOML serialize → deserialize → assert_eq) for all types
- Backward compat tests (legacy TOML without new fields parses cleanly)

### Integration Points
- `GatesSpec` struct gains 3 new optional fields
- New types (`CriteriaLibrary`, `SpecPreconditions`, `PreconditionStatus`, `RequireStatus`, `CommandStatus`) added to `assay-types`
- Schema snapshots in `crates/assay-types/tests/snapshots/` need updating
- `lib.rs` re-exports for new public types
- `semver` crate added as workspace dependency in root `Cargo.toml`

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Follow existing assay-types conventions throughout.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 64-type-foundation*
*Context gathered: 2026-04-11*
