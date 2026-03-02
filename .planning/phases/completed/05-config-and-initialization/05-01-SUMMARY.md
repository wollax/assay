# Phase 5 Plan 01: Config Type Redesign and Error Variants Summary

**One-liner:** Redesigned Config to `{project_name, specs_dir, gates}` with GatesConfig and three new AssayError variants for config parsing, validation, and init

## Frontmatter

- **Phase:** 05-config-and-initialization
- **Plan:** 01
- **Subsystem:** types, error-handling
- **Tags:** config, gates-config, error-variants, serde, deny-unknown-fields, schema
- **Completed:** 2026-03-02
- **Duration:** ~12 minutes

### Dependency Graph

- **Requires:** Phase 3 (AssayError enum, domain model), Phase 4 (schema generation pipeline)
- **Provides:** Redesigned Config/GatesConfig types, ConfigError type, ConfigParse/ConfigValidation/AlreadyInitialized error variants
- **Affects:** Phase 5 Plans 02-03 (config loading/validation/init use these types and errors)

### Tech Stack

- **Added:** `toml` dependency to assay-core (already in workspace, now in assay-core's `[dependencies]`)
- **Patterns:** `#[serde(deny_unknown_fields)]` for strict TOML deserialization; `ConfigError` as validation-specific error type imported into `AssayError`

### Key Files

**Created:**
- `schemas/gates-config.schema.json`
- `crates/assay-types/tests/snapshots/schema_snapshots__gates-config-schema.snap`

**Modified:**
- `crates/assay-types/src/lib.rs` — Config redesign, GatesConfig addition
- `crates/assay-core/src/error.rs` — Three new AssayError variants
- `crates/assay-core/src/config/mod.rs` — ConfigError type with Display
- `crates/assay-core/Cargo.toml` — toml dependency
- `crates/assay-types/tests/schema_roundtrip.rs` — Updated config test, added gates-config test
- `crates/assay-types/tests/schema_snapshots.rs` — Added gates-config snapshot test
- `crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap` — Updated for new Config shape
- `schemas/config.schema.json` — Updated for new Config shape

### Decisions

| Decision | Rationale |
|----------|-----------|
| ConfigError in config/mod.rs, not error.rs | Config-specific validation output stays with config concerns; imported into error.rs for the ConfigValidation variant |
| toml dep added to assay-core now | Error variants reference config types that will use toml in Plan 02; dependency needed for the module to compile with future additions |
| Existing Workflow/Gate types left untouched | Only Config replaced; placeholder types will be revisited in later phases |

### Metrics

- **Tasks:** 2/2 complete
- **Tests:** 27 passed (assay-types), 3 passed (assay-core)
- **Schemas:** 9 generated (config and gates-config updated/created)

## Task Summary

| Task | Name | Commit | Key Changes |
|------|------|--------|-------------|
| 1 | Redesign Config type and add GatesConfig | 9424c16 | Config redesigned with project_name/specs_dir/gates, GatesConfig added, deny_unknown_fields on both, inventory submit entries |
| 2 | Add error variants, update tests/snapshots/schemas | 6ddfb8c | ConfigError type, 3 AssayError variants, toml dep, updated tests/snapshots/schemas |

## Deviations from Plan

None -- plan executed exactly as written.

## Verification Results

1. `just ready` passes -- all fmt, lint, test, deny checks green
2. `cargo test -p assay-types -- config` passes -- both roundtrip and snapshot tests
3. `schemas/config.schema.json` contains `specs_dir` and `gates` fields, no `workflows`, has `additionalProperties: false`
4. `schemas/gates-config.schema.json` exists with `default_timeout` and `working_dir`, has `additionalProperties: false`
5. `cargo check -p assay-core` compiles with the three new error variants

## Next Phase Readiness

Plan 05-02 (config loading and validation via TDD) is unblocked. The Config/GatesConfig types, ConfigError, and all three AssayError variants are in place. The `toml` dependency is available in assay-core.
