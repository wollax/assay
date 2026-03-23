# S01: Manifest Extension

**Goal:** Extend `JobManifest` with `ComposeService` type and `Vec<ComposeService>` services field, extend `validate()` with compose-specific rules, and prove correctness with a complete test suite.
**Demo:** `cargo test -p smelt-core` passes all roundtrip and validation tests for `[[services]]`; `cargo test --workspace` shows zero regressions; `smelt run --dry-run` accepts a compose manifest without errors.

## Must-Haves

- `ComposeService` struct with `name: String`, `image: String`, and `#[serde(flatten)] extra: IndexMap<String, toml::Value>` parses correctly from TOML `[[services]]` entries
- `extra` does NOT contain `name` or `image` keys — serde flatten behavior is correct
- `JobManifest.services` defaults to empty `Vec` (backward compat — all existing manifests parse unchanged)
- `validate()` rejects `runtime` values other than `"docker"` or `"compose"` (catches typos early per D018)
- `validate()` rejects non-empty `services` when `runtime != "compose"`
- `validate()` rejects `ComposeService` entries with empty `name` or `image` when `runtime == "compose"`
- `validate()` allows empty `services` list when `runtime == "compose"` (zero services is valid)
- `cargo test --workspace` — zero regressions in all crates

## Proof Level

- This slice proves: contract
- Real runtime required: no
- Human/UAT required: no

## Verification

```
cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"
cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
```

All test results must show `ok`. No `FAILED` lines.

The critical new tests are in `crates/smelt-core/src/manifest.rs`:
- `test_compose_manifest_roundtrip_with_services` — parse and assert name, image, extra keys
- `test_compose_service_extra_does_not_contain_name_or_image` — serde flatten contract
- `test_compose_service_passthrough_types` — extra fields include int, bool, array (type fidelity)
- `test_validate_compose_service_missing_name` — validation error path
- `test_validate_compose_service_missing_image` — validation error path
- `test_validate_services_require_compose_runtime` — docker + services = error
- `test_validate_compose_empty_services_allowed` — compose + no services = valid
- `test_validate_runtime_unknown_rejected` — unknown runtime rejected
- `test_validate_runtime_compose_valid` — compose runtime + services passes validate()

## Observability / Diagnostics

- Runtime signals: None — this is synchronous TOML parsing with no async state
- Inspection surfaces: `validate()` returns a collected error list per D018; all errors visible in the `SmeltError::Manifest` message
- Failure visibility: each validation error is a descriptive string pushed to `errors: Vec<String>`; displayed all-at-once to the caller
- Redaction constraints: None — manifests contain no secrets at parse time

## Integration Closure

- Upstream surfaces consumed: none (S01 is independent)
- New wiring introduced in this slice: `ComposeService` type and `JobManifest.services: Vec<ComposeService>` — the S01→S02/S03/S04 boundary contract per the roadmap
- What remains before the milestone is truly usable end-to-end: S02 (compose file generation), S03 (ComposeProvider lifecycle), S04 (CLI dispatch + dry-run section)

## Tasks

- [x] **T01: Add ComposeService struct, indexmap dependency, and services field** `est:30m`
  - Why: Establishes the data model that S02/S03/S04 consume. Without this struct and field, nothing downstream compiles.
  - Files: `Cargo.toml` (workspace), `crates/smelt-core/Cargo.toml`, `crates/smelt-core/src/manifest.rs`
  - Do: Add `indexmap = { version = "2", features = ["serde"] }` to workspace deps; add `indexmap.workspace = true` to smelt-core deps; add `use indexmap::IndexMap;` import; define `ComposeService` struct with `name`, `image`, and `#[serde(flatten)] extra: IndexMap<String, toml::Value>` — NO `deny_unknown_fields`; add `#[serde(default)] pub services: Vec<ComposeService>` to `JobManifest`
  - Verify: `cargo build -p smelt-core` succeeds; `cargo test -p smelt-core --lib` shows 121 passed, 0 failed (zero regressions)
  - Done when: `JobManifest` compiles with the services field; all 121 existing tests pass

- [x] **T02: Extend validate() with compose rules and write full test suite** `est:45m`
  - Why: The data model alone is not useful unless validation enforces the invariants the boundary map specifies; tests prove the contract for downstream slices.
  - Files: `crates/smelt-core/src/manifest.rs`
  - Do: Add runtime value check in `validate()` (must be "docker" or "compose"); add services-with-wrong-runtime check; add per-service name/image non-empty checks for compose runtime; write all 9 new tests listed in Verification including a type-fidelity test with int/bool/array extra fields; assert `extra` does NOT contain "name" or "image"
  - Verify: `cargo test -p smelt-core --lib` passes all new and existing tests; `cargo test --workspace` shows zero regressions
  - Done when: All 9 new tests pass; existing 121 tests still pass; `cargo test --workspace` clean

## Files Likely Touched

- `Cargo.toml` (workspace — add indexmap dep)
- `crates/smelt-core/Cargo.toml` (add indexmap)
- `crates/smelt-core/src/manifest.rs` (ComposeService struct, services field, validate() extensions, tests)
