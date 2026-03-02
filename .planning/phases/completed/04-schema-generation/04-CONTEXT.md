# Phase 4: Schema Generation - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Produce JSON Schema files from domain types so external tools and agents can validate Assay config and spec formats. One requirement: FND-07 (schema generation binary + `just schemas` recipe). Does not add new types — consumes types from Phase 3 and future phases.

</domain>

<decisions>
## Implementation Decisions

### Schema scope
- Generator should auto-discover all schemars-derived types — no hardcoded list, so future types added in Phases 5-6 are picked up automatically on re-run
- Roundtrip validation (serialize known-good instance to JSON, validate against schema) + snapshot tests (determinism) — both required
- Draft version: JSON Schema Draft 2020-12

### File organization
- Claude's discretion on flat vs grouped layout and naming convention (kebab-case vs snake_case)
- Claude's discretion on generator approach (example binary as roadmap specifies vs alternatives)
- Claude's discretion on overwrite vs diff-check behavior for `just schemas`

### Version control and usage
- Schemas should include rich metadata: `$id`, `title`, `description`, `$schema` draft version — self-documenting and standalone
- All four consumer classes are relevant: AI agents (MCP), IDE/editor validation, CI validation, and documentation
- Claude's discretion on whether schemas are committed to git or gitignored

### Claude's Discretion
- Which specific types get individual schema files vs inlined (top-level only vs all public types)
- Whether runtime types (GateResult) get schemas alongside config types
- File layout (flat vs grouped) and naming convention
- Generator implementation approach
- `just schemas` overwrite/check behavior
- Git tracking of generated schemas

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. User gave broad latitude on implementation details, with two firm decisions:

1. Auto-discovery of types (not a hardcoded list) so future phases benefit without touching the generator
2. Both roundtrip validation AND snapshot testing for schema correctness and determinism

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 04-schema-generation*
*Context gathered: 2026-03-01*
