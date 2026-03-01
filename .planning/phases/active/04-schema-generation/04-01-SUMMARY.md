# Phase 4 Plan 01: Schema Generation Pipeline Summary

**One-liner:** Inventory-based auto-discovery registry + generator binary producing 8 Draft 2020-12 JSON Schema files with roundtrip validation and snapshot determinism tests.

---

phase: 04-schema-generation
plan: 01
subsystem: schema-generation
tags: [json-schema, schemars, inventory, insta, jsonschema, codegen]

requires:
  - phase-03 (domain types: GateKind, GateResult, Criterion, Spec, Gate, Review, Workflow, Config)

provides:
  - Schema registry with inventory-based auto-discovery (SchemaEntry + all_entries)
  - Generator binary producing per-type JSON Schema files
  - `just schemas` and `just schemas-check` recipes
  - Roundtrip validation test infrastructure (jsonschema Draft 2020-12)
  - Snapshot determinism test infrastructure (insta)

affects:
  - Future phases adding new types — derive JsonSchema + inventory::submit! automatically generates schemas
  - Phase 8 (MCP) — agents can validate config/spec formats against published schemas
  - Phase 10 (Plugin) — IDE/editor JSON validation uses committed schema files

tech-stack:
  added:
    - inventory 0.3 (type auto-discovery via linker sections)
    - insta 1.46 with json feature (snapshot testing)
    - jsonschema 0.43 (Draft 2020-12 validation in tests)
  patterns:
    - Distributed registration: inventory::submit! at type definition site
    - Convention: every JsonSchema-derived type MUST have inventory::submit! immediately after definition
    - Generator as cargo example binary (not build.rs, not separate binary crate)
    - Metadata enrichment: $id injected post-generation for self-documenting schemas

key-files:
  created:
    - crates/assay-types/src/schema_registry.rs
    - crates/assay-types/examples/generate-schemas.rs
    - crates/assay-types/tests/schema_roundtrip.rs
    - crates/assay-types/tests/schema_snapshots.rs
    - crates/assay-types/tests/snapshots/ (8 snapshot files)
    - schemas/config.schema.json
    - schemas/criterion.schema.json
    - schemas/gate.schema.json
    - schemas/gate-kind.schema.json
    - schemas/gate-result.schema.json
    - schemas/review.schema.json
    - schemas/spec.schema.json
    - schemas/workflow.schema.json
  modified:
    - Cargo.toml (added inventory, insta, jsonschema workspace deps)
    - crates/assay-types/Cargo.toml (added inventory dep, insta/jsonschema dev-deps)
    - crates/assay-types/src/lib.rs (added schema_registry module, inventory::submit! for 5 types)
    - crates/assay-types/src/gate.rs (added inventory::submit! for GateKind, GateResult)
    - crates/assay-types/src/criterion.rs (added inventory::submit! for Criterion)
    - justfile (added schemas and schemas-check recipes)

decisions:
  - inventory::iter returns IntoIterator, not Iterator — all_entries() calls .into_iter()
  - Examples CAN access dev-dependencies in Rust (confirmed empirically — serde_json stays as dev-dep)
  - schemas-check NOT added to `just ready` recipe to avoid circular dependency during development
  - All 8 public types get individual schema files (not just top-level Config)
  - Schema $id uses https://assay.dev/schemas/{name}.schema.json convention (aspirational, not resolvable)
  - Schemas committed to git for IDE/consumer access without building

metrics:
  duration: ~7 minutes
  completed: 2026-03-01
  tasks: 2/2
  tests-added: 19 (11 roundtrip + 8 snapshot)
  tests-total: 25 (6 existing + 19 new)
  schemas-generated: 8

---

## Tasks Completed

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Add dependencies, create schema registry, register all types | 1c9b3be | schema_registry.rs, inventory deps, 8 type registrations |
| 2 | Generator binary, just recipes, roundtrip and snapshot tests | ac6e814 | generate-schemas.rs, justfile recipes, 19 tests, 8 schema files |

## Decisions Made

1. **inventory::iter API:** `inventory::iter::<T>` returns an `IntoIterator`, not `Iterator`. The `all_entries()` function calls `.into_iter()` to provide an `impl Iterator` return type.

2. **Examples use dev-dependencies:** Confirmed that Rust cargo examples CAN access dev-dependencies. `serde_json` remains a dev-dependency of assay-types — no need to promote it.

3. **schemas-check not in ready:** The `just schemas-check` recipe is not added to `just ready` to avoid circular dependency during active development. It can be added to CI separately.

4. **All public types get schemas:** Every type deriving `JsonSchema` gets its own schema file via inventory registration, including types that also appear as `$defs` inside parent schemas (e.g., Spec appears both as `spec.schema.json` and inside `config.schema.json`'s `$defs`).

5. **Schema $id convention:** Using `https://assay.dev/schemas/{name}.schema.json` as the `$id` URI. This is aspirational (domain doesn't need to exist) but follows JSON Schema conventions and can become resolvable if schemas are published.

## Deviations from Plan

None — plan executed exactly as written.

## Verification Results

- `just schemas` produces 8 `.schema.json` files in `schemas/`
- Each schema contains `$schema` (Draft 2020-12), `$id`, `title`, and `description`
- `just schemas-check` passes (regeneration produces byte-identical output)
- `cargo test -p assay-types` passes all 25 tests (6 original + 11 roundtrip + 8 snapshot)
- Running `just schemas` twice produces identical output (determinism confirmed)
- `just ready` passes (fmt-check + lint + test + deny)
- Schema files end with trailing newline

## Test Coverage

| Test File | Tests | What It Covers |
|-----------|-------|---------------|
| schema_roundtrip.rs | 11 | Validates known-good instances of all 8 types against Draft 2020-12 schemas |
| schema_snapshots.rs | 8 | Snapshot locks schema output for determinism detection |
| gate.rs (existing) | 4 | TOML roundtrip + JSON field skip behavior |
| criterion.rs (existing) | 2 | TOML roundtrip for cmd present/absent |

## Next Phase Readiness

Phase 4 is complete (single plan). The schema generation pipeline is fully operational:
- New types added in Phases 5-6 only need `#[derive(JsonSchema)]` + `inventory::submit!` to get schemas
- No changes to the generator binary required
- `just schemas` regenerates all schema files
- CI can use `just schemas-check` to verify schemas are fresh
