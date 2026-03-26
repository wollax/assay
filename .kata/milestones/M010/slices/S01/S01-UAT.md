# S01: StateBackend trait and CapabilitySet — UAT

**Milestone:** M010
**Written:** 2026-03-26

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is pure type definitions and stubs with no runtime behavior. All verification is contract-level — compile-time object-safety, unit tests for flag constructors, stub method returns. No agent invocation, no filesystem side effects, no server lifecycle. Artifact-driven UAT (locked snapshot + passing tests) is the correct and complete verification mode for this slice.

## Preconditions

- Rust toolchain installed (`cargo`, `cargo nextest`)
- Branch `kata/root/M010/S01` checked out
- `just` installed

## Smoke Test

```
cargo test -p assay-core --features orchestrate --test state_backend
```
All 6 contract tests pass in < 1 second.

## Test Cases

### 1. StateBackendConfig schema snapshot locked

1. Run `cargo test -p assay-types --test schema_snapshots state_backend_config_schema_snapshot`
2. **Expected:** Test passes; snapshot file exists at `crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap`

### 2. CapabilitySet constructors

1. Run `cargo test -p assay-core --features orchestrate --test state_backend test_capability_set_all test_capability_set_none`
2. **Expected:** Both pass. `all()` returns all four flags true; `none()` returns all four flags false.

### 3. LocalFsBackend as trait object

1. Run `cargo test -p assay-core --features orchestrate --test state_backend test_local_fs_backend_as_trait_object`
2. **Expected:** Passes. `Box<dyn StateBackend>` construction compiles and `capabilities().supports_messaging` is true.

### 4. LocalFsBackend stub methods return expected values

1. Run `cargo test -p assay-core --features orchestrate --test state_backend`
2. **Expected:** `push_session_event` returns `Ok(())`; `read_run_state` returns `Ok(None)`.

### 5. Full workspace — zero regressions

1. Run `just ready`
2. **Expected:** fmt + lint + test (1471 tests) + deny all green. Zero failures.

## Edge Cases

### Object-safety compile guard

1. Inspect `crates/assay-core/src/state_backend.rs` for `fn _assert_object_safe`
2. **Expected:** Private function present with `#[allow(dead_code)]`. Removing any default-returning method from the trait would cause this function to fail compilation.

### Feature gate

1. Run `cargo test -p assay-core --test state_backend` (without `--features orchestrate`)
2. **Expected:** The test file is excluded by `#![cfg(feature = "orchestrate")]`; 0 tests run from that file (no failure).

## Failure Signals

- `state_backend_config_schema_snapshot` fails → snapshot out of date or `StateBackendConfig` type changed
- `test_local_fs_backend_as_trait_object` fails → object-safety violated, likely a non-object-safe method added to the trait
- `gate_finalize_*` tests fail → CWD mismatch; tests missing `#[serial]` + `create_project()`
- `just ready` non-green → fmt/clippy/deny regression introduced

## Requirements Proved By This UAT

- R071 (StateBackend trait and CapabilitySet) — trait defined with correct method signatures, object safety proven at compile time via `_assert_object_safe`, `CapabilitySet::all()`/`none()` constructors verified, `StateBackendConfig` serde schema locked by snapshot

## Not Proven By This UAT

- Real filesystem reads/writes via `LocalFsBackend` — all methods are stubs; real implementations land in S02
- Orchestrator wiring (`OrchestratorConfig.backend`) — S02
- `RunManifest.state_backend` field backward compatibility — S02
- CapabilitySet graceful degradation paths — S03
- smelt-agent plugin usability — S04 (human UAT)
- Remote backends (Linear, GitHub, SSH) — M011+

## Notes for Tester

The two `gate_finalize_*` tests in `assay-mcp` previously failed under `cargo nextest` due to missing `#[serial]` + `create_project()` setup. This was fixed in the final commit of this slice. If you see those tests fail, ensure you're on the latest commit on `kata/root/M010/S01`.

The `run_manifest_schema_snapshot` test was pre-existing-failing before this slice and is unrelated to M010/S01 work.
