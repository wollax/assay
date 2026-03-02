---
phase: 04-schema-generation
verified_by: kata-verifier
verified_at: 2026-03-01
status: PASS
---

# Phase 04 Verification Report

**Goal**: Produce JSON Schema files from domain types so external tools and agents can validate Assay config and spec formats.

**Verdict: PASS** — All must-haves are satisfied. Goal is fully achieved.

---

## Must-Have Truths

### T1: `just schemas` produces one JSON Schema file per registered type in `schemas/`

**PASS**

8 types registered via `inventory::submit!` across three files:
- `crates/assay-types/src/lib.rs`: `Spec`, `Gate`, `Review`, `Workflow`, `Config` (5 entries)
- `crates/assay-types/src/gate.rs`: `GateKind`, `GateResult` (2 entries)
- `crates/assay-types/src/criterion.rs`: `Criterion` (1 entry)

Running `just schemas` produces exactly 8 files in `schemas/`:
```
config.schema.json  criterion.schema.json  gate-kind.schema.json  gate-result.schema.json
gate.schema.json    review.schema.json     spec.schema.json       workflow.schema.json
```

### T2: Running `just schemas` twice produces byte-identical output (determinism)

**PASS**

Verified by: snapshot the schemas directory, re-running `just schemas`, then `diff -r`. Output was identical. `just schemas-check` also passed cleanly:
```
Schemas are up to date.
```

### T3: Generated schemas validate known-good instances of each type (roundtrip)

**PASS**

`crates/assay-types/tests/schema_roundtrip.rs` contains 11 tests covering all registered types including edge cases (minimal/full `GateResult`, `Criterion` with/without `cmd`, `GateKind::Command` and `AlwaysPass`). All 11 tests pass. Validator uses `jsonschema::draft202012::new`.

### T4: Adding a new type with `#[derive(JsonSchema)]` + `inventory::submit!` automatically generates its schema without touching the generator

**PASS**

`crates/assay-types/examples/generate-schemas.rs` contains zero hardcoded type references. The generator body is:
```rust
for entry in schema_registry::all_entries() {
    let mut schema = (entry.generate)();
    // ... write to file
}
```
No mention of `GateKind`, `GateResult`, `Criterion`, `Spec`, `Gate`, `Review`, `Workflow`, or `Config` anywhere in the generator. New types are discovered purely via `inventory::collect!` / `inventory::iter`.

### T5: Each schema file contains `$schema`, `$id`, `title`, and is valid Draft 2020-12

**PASS**

All 8 schema files verified:

| File | `$schema` | `$id` | `title` |
|------|-----------|-------|---------|
| config.schema.json | `https://json-schema.org/draft/2020-12/schema` | present | `Config` |
| criterion.schema.json | present | present | `Criterion` |
| gate-kind.schema.json | present | present | `GateKind` |
| gate-result.schema.json | present | present | `GateResult` |
| gate.schema.json | present | present | `Gate` |
| review.schema.json | present | present | `Review` |
| spec.schema.json | present | present | `Spec` |
| workflow.schema.json | present | present | `Workflow` |

`$id` format: `https://assay.dev/schemas/{name}.schema.json` — injected by the generator at write time (not from schemars, which does not produce `$id`).

---

## Artifact Verification

### `crates/assay-types/src/schema_registry.rs`

**PASS** — File exists, contains `SchemaEntry` struct, `inventory::collect!(SchemaEntry)`, and `pub fn all_entries()`.

### `crates/assay-types/examples/generate-schemas.rs`

**PASS** — File exists. Calls `schema_registry::all_entries()`, injects `$id`, writes one file per entry with a trailing newline for determinism.

### `schemas/`

**PASS** — Directory exists with 8 JSON Schema files and a `README.md`.

### `justfile`

**PASS** — Contains both `schemas` and `schemas-check` recipes:
- `schemas`: `cargo run -p assay-types --example generate-schemas`
- `schemas-check`: bash script that snapshots schemas, regenerates, and diffs

### `crates/assay-types/tests/schema_roundtrip.rs`

**PASS** — File exists, contains `jsonschema::draft202012::new`, 11 tests covering all 8 registered types.

### `crates/assay-types/tests/schema_snapshots.rs`

**PASS** — File exists, contains `assert_json_snapshot!`, 8 tests (one per type). All 8 snapshot files present under `tests/snapshots/`.

---

## Key Link Verification

| Link | Expected Pattern | Status |
|------|-----------------|--------|
| `gate.rs` → `schema_registry.rs` | `inventory::submit!` for `GateKind` and `GateResult` | PASS |
| `criterion.rs` → `schema_registry.rs` | `inventory::submit!` for `Criterion` | PASS |
| `generate-schemas.rs` → `schema_registry.rs` | `schema_registry::all_entries()` | PASS |
| `justfile` → `generate-schemas.rs` | `generate-schemas` in recipe body | PASS |

---

## Requirements Coverage

### FND-07: Schema generation binary + `just schemas` recipe

**PASS** — Binary at `crates/assay-types/examples/generate-schemas.rs`. Recipe `just schemas` present and functional.

---

## ROADMAP Success Criteria

1. **`just schemas` produces JSON Schema files in `schemas/` for all public domain types** — PASS (8 schemas, 8 registered types)
2. **Generated schemas reflect schemars 1.x output and validate against sample TOML-converted-to-JSON input** — PASS (schemars 1.2.1 in lockfile, 11 roundtrip validation tests passing with `jsonschema::draft202012`)
3. **Schema files are deterministic (re-running produces identical output)** — PASS (verified by diff, confirmed by `just schemas-check`)

---

## Full Suite Status

`just ready` (fmt-check + lint + test + deny): **All checks passed.**

- 25 tests across 4 suites, 0 failures
- clippy: 0 warnings
- cargo-deny: advisories ok, bans ok, licenses ok, sources ok
