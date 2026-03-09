# Plan 27-04 Summary: Criterion Dedup

**Status:** Complete
**Started:** 2026-03-09T20:59:18Z
**Completed:** 2026-03-09T21:14:23Z
**Duration:** ~15 minutes

## Objective

Merge `GateCriterion` into `Criterion` by adding a `requirements` field to `Criterion`, then make `GateCriterion` a backward-compatible type alias.

## Tasks Completed

### Task 1: Add requirements field and create type alias
- Added `requirements: Vec<String>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` to `Criterion`
- Replaced `GateCriterion` struct with `pub type GateCriterion = crate::Criterion;`
- Updated `gate-criterion` schema registry to use `crate::Criterion`
- Updated all test code in assay-types (criterion.rs, gates_spec.rs, schema_roundtrip.rs)
- Updated insta schema snapshots (criterion, gate-criterion, gates-spec, spec, workflow)

### Task 2: Update assay-core and assay-mcp
- Simplified `to_criterion()` from field-by-field copy to `gc.clone()`
- Added `requirements: vec![]` to all `Criterion` struct literals across assay-core (gate/mod.rs, spec/mod.rs) and assay-mcp (server.rs)
- Confirmed `CriterionResult` naming is appropriate (names the result of evaluating a criterion)

### Task 3: Schema registry verification
- Verified `criterion` schema includes optional `requirements` array field
- Verified `criterion` and `gate-criterion` schemas produce identical output
- Verified `gates-spec` schema references the merged `Criterion` type correctly

## Commits

| Hash | Message |
|------|---------|
| `2b126ed` | refactor(27-04): merge GateCriterion into Criterion with type alias |
| `9838072` | refactor(27-04): update assay-core and assay-mcp for merged Criterion type |

## Deviations

- **assay-mcp/src/server.rs** also needed `requirements: vec![]` additions (3 test Criterion literals). Not listed in plan files but required for workspace compilation.

## Verification

- `just fmt-check` — pass
- `just lint` — pass
- `just test` — pass (522 tests, 3 ignored)
- `just deny` — pass
- `just ready` — pre-existing `check-plugin-version` failure (unrelated to this plan)
