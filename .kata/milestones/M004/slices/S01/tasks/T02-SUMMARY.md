---
id: T02
parent: S01
milestone: M004
provides:
  - "`validate()` rejects unknown runtime values with error containing `environment.runtime`"
  - "`validate()` rejects `[[services]]` entries when `runtime != \"compose\"` with error containing `services:`"
  - "`validate()` validates per-service `name`/`image` non-empty when `runtime = \"compose\"`"
  - "`VALID_COMPOSE_MANIFEST` test constant with two services (postgres with all extra types, redis bare)"
  - "10 new tests proving all compose validation invariants and serde flatten boundary contract (131 total)"
key_files:
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - "Struct literal fix in docker_lifecycle.rs: added `services: vec![]` to `test_manifest_with_repo()` after new field was added in T01 — required to keep workspace compiling"
patterns_established:
  - "Compose validation is conditionally applied: service name/image checks only run when `runtime == \"compose\"`; runtime check and services-require-compose check run unconditionally"
  - "`VALID_COMPOSE_MANIFEST` covers all four extra-field TOML types (string, integer, boolean, array) in a single constant — reusable for future tests"
observability_surfaces:
  - "`cargo test -p smelt-core --lib --test test_validate_runtime_unknown_rejected` — spot-checks runtime validation path"
  - "Each validation error is a descriptive string in `SmeltError::Manifest { message }` per D018 (collect-all-errors)"
duration: 10min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T02: Extend validate() with compose rules and write full test suite

**Extended `validate()` with three compose-specific rules and wrote 10 tests (131 total) proving the complete S01 boundary contract — runtime validation, services-require-compose, per-service name/image checks, serde flatten exclusion, and all four extra-field TOML types.**

## What Happened

Added three new validation blocks to `JobManifest::validate()`:

1. **Runtime allowlist** (after `environment.image` check): rejects any `runtime` value not in `["docker", "compose"]` — error contains `"environment.runtime"`.

2. **Services-require-compose** (immediately after runtime check): rejects non-empty `services` when `runtime != "compose"` — error contains `"services:"` and mentions the actual runtime value.

3. **Per-service name/image** (after cycle detection block, before `merge.target` check): iterates `self.services` when `runtime == "compose"` and rejects empty `name` or `image` — errors contain `"services[N].name"` / `"services[N].image"`.

Added `VALID_COMPOSE_MANIFEST` constant with two `[[services]]` entries: `postgres` (with `port = 5432`, `restart = true`, `command = [...]`, `tag = "db"` to exercise all four TOML extra types) and `redis` (bare).

Wrote 10 tests (the 9 from the task plan plus `test_validate_runtime_compose_valid` from the slice plan):
- `test_compose_manifest_roundtrip_with_services` — parse, assert len/name/image/extra keys/no name|image in extra
- `test_compose_manifest_roundtrip_no_services` — docker manifest has empty services vec
- `test_compose_service_extra_does_not_contain_name_or_image` — serde flatten contract
- `test_compose_service_passthrough_types` — Integer(5432), Boolean(true), Array([...]), String
- `test_validate_compose_service_missing_name` — error contains `services[0].name`
- `test_validate_compose_service_missing_image` — error contains `services[0].image`
- `test_validate_services_require_compose_runtime` — docker + services = error contains `services:` and `compose`
- `test_validate_compose_empty_services_allowed` — compose + no services = `Ok(())`
- `test_validate_runtime_unknown_rejected` — kubernetes → error contains `environment.runtime`
- `test_validate_runtime_compose_valid` — compose + services passes `validate()`

Also fixed a compile error in `crates/smelt-cli/tests/docker_lifecycle.rs`: the `test_manifest_with_repo()` helper constructed `JobManifest` via struct literal and was missing the new `services` field — added `services: vec![]`.

## Verification

```
cargo test -p smelt-core --lib 2>&1 | tail -3
  → test result: ok. 131 passed; 0 failed

cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
  → 8 × "test result: ok. N passed; 0 failed"  (no FAILED lines)
```

## Diagnostics

- `cargo test -p smelt-core --lib --test test_validate_runtime_unknown_rejected` — spot-checks runtime validation
- All validation errors flow through `SmeltError::Manifest { message }` per D018; full collected error list visible in the error message

## Deviations

One extra test added: `test_validate_runtime_compose_valid` (listed in the slice plan's Verification section but not in the task plan's step list). Added proactively since the slice plan called for it and it covers the compose-valid happy path.

Compile fix required in `docker_lifecycle.rs` — not in the task plan but necessary to keep `cargo test --workspace` passing. Low-risk mechanical change: `services: vec![]` on existing struct literal.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/manifest.rs` — three new validation blocks in `validate()`; `VALID_COMPOSE_MANIFEST` constant; 10 new test functions
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added `services: vec![]` to `test_manifest_with_repo()` struct literal
