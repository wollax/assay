# Phase 65: Resolution Core - Context

**Gathered:** 2026-04-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Criteria library file I/O (load, save, scan from `.assay/criteria/`) and `spec::compose::resolve()` that merges parent criteria + library criteria + own criteria into a `ResolvedGate` with per-criterion source tracking, cycle detection, and slug validation. Single-level extends only (multi-level deferred to INHR-05).

</domain>

<decisions>
## Implementation Decisions

### Source annotation shape
- Claude's discretion on annotation detail level (simple origin enum vs origin + override chain)
- `ResolvedCriterion` and `ResolvedGate` types live in `assay-types` (not assay-core) — MCP tools (Phase 68: spec_resolve) need to return them
- `ResolvedGate` struct wraps criteria list with metadata: parent name, included libraries
- All new types get schemars `JsonSchema` derives immediately — consistent with all assay-types structs, prevents Phase 68 churn

### resolve() caller contract
- Closure pair: `load_gate: impl Fn(&str) -> Result<GatesSpec>` and `load_library: impl Fn(&str) -> Result<CriteriaLibrary>` — lazy loading, zero-trait convention
- Single-level extends only: A extends B, B's extends is ignored. Max chain depth 2. Multi-level deferred to INHR-05.
- Child's `include` libraries only — parent's includes are not re-resolved (parent criteria are taken as-is)
- Lives in `spec::compose` module (`crates/assay-core/src/spec/compose.rs`)

### Slug validation
- New `validate_slug()` function: pattern `^[a-z0-9][a-z0-9_-]*$`, max 64 chars
- Allows hyphens AND underscores (permits both `my-lib` and `my_lib`)
- Applied to: library slugs, extends values, include values — AND retroactively to spec slugs and gate names
- `pub fn` visibility — downstream phases (wizard, MCP, TUI) all need slug validation
- Replaces `validate_path_component` for slug contexts (path_component stays for backwards-compat on non-slug identifiers)

### Error handling
- Structured per-error AssayError variants: `CycleDetected`, `LibraryNotFound`, `ParentGateNotFound`, `InvalidSlug` — consistent with existing error pattern
- Fail on first error (not collecting all errors) — consistent with existing load/parse functions
- Fuzzy suggestions on not-found errors ("did you mean 'lint-rules'?") — reuse existing fuzzy matching pattern from gate/spec errors

### Claude's Discretion
- Source annotation detail level (simple enum vs origin + override chain)
- Whether precondition command output reuses existing head+tail truncation
- Exact serde field ordering and skip_serializing_if patterns (follow existing conventions)
- Library I/O atomicity (tempfile-then-rename pattern from work_session, or simpler direct write)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `validate_path_component` (`history/mod.rs:27`): Existing path safety check — slug validation extends this pattern
- `save_session`/`load_session` (`work_session.rs`): Atomic JSON persistence pattern (tempfile → rename) — reuse for library save
- `spec::scan()` (`spec/mod.rs:683`): Directory scanning pattern for specs — reuse for library scanning
- `spec::load_gates()` (`spec/mod.rs:400`): TOML loading pattern for gate files — reuse for library loading
- `enriched_error_display` (`gate/mod.rs:474`): Fuzzy matching on errors — reuse for slug suggestions
- `CriteriaLibrary` (`assay-types/src/criteria_library.rs`): Type already exists from Phase 64

### Established Patterns
- Free functions, not methods or traits (zero-trait convention)
- `#[serde(default, skip_serializing_if)]` on all optional fields
- `#[serde(deny_unknown_fields)]` on TOML-authored types
- `inventory::submit!` for schema registration
- Inline TDD tests with `#[cfg(test)]` modules
- `AssayError` is `#[non_exhaustive]` with structured variants carrying context fields

### Integration Points
- `spec::compose` new submodule under `crates/assay-core/src/spec/`
- `ResolvedGate`, `ResolvedCriterion`, `CriterionSource` types added to `assay-types`
- New `AssayError` variants: `CycleDetected`, `LibraryNotFound`, `ParentGateNotFound`, `InvalidSlug`
- Library files stored at `.assay/criteria/<slug>.toml`

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Follow existing assay-core conventions throughout.

</specifics>

<deferred>
## Deferred Ideas

- Multi-error collection in resolve() (return all errors at once instead of fail-on-first) — future milestone, noted for backlog
- Multi-level inheritance INHR-05 (> 2 levels with configurable depth limit) — already in REQUIREMENTS.md Future section
- Parameterized/template criteria CLIB-05 — already in REQUIREMENTS.md Future section

</deferred>

---

*Phase: 65-resolution-core*
*Context gathered: 2026-04-11*
