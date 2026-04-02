---
id: T01
parent: S02
milestone: M001
provides:
  - bollard and futures-util workspace dependencies
  - DockerProvider struct with Docker daemon connection
  - parse_memory_bytes and parse_cpu_nanocpus resource parsing utilities
  - Integration test scaffolding for Docker lifecycle (skip-when-no-daemon)
key_files:
  - crates/smelt-core/src/docker.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - Docker lifecycle integration tests skip gracefully (return early) when Docker daemon is unavailable rather than panicking — keeps cargo test --workspace green in all environments
patterns_established:
  - docker_provider_or_skip() helper returns Option<DockerProvider> for test skip pattern
  - Resource parsing uses SmeltError::Provider with operation names "parse_memory" and "parse_cpu"
  - DockerProvider exposes client() accessor for test assertions via bollard inspect APIs
observability_surfaces:
  - SmeltError::Provider wraps bollard connection errors with operation="connect" and source chain
  - cargo test -p smelt-core -- docker::tests runs 16 resource parsing unit tests
duration: 12m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add bollard dependency, resource parser, and integration test scaffolding

**Added bollard/futures-util deps, DockerProvider struct with resource parsers (16 unit tests), and 4 Docker lifecycle integration tests that compile and skip when daemon unavailable.**

## What Happened

Added `bollard = "0.20"` and `futures-util = "0.3"` to workspace dependencies and wired them into smelt-core. Added tokio and bollard to smelt-cli dev-dependencies for async integration tests.

Created `crates/smelt-core/src/docker.rs` with:
- `DockerProvider` struct holding a `bollard::Docker` client
- `DockerProvider::new()` connecting via socket defaults, wrapping errors with `SmeltError::provider_with_source()`
- `parse_memory_bytes()` handling G/M/K suffixes and plain bytes (case-insensitive, fractional values)
- `parse_cpu_nanocpus()` handling integer and decimal CPU values (multiply by 1B nanoseconds)
- 16 unit tests covering valid inputs, edge cases (whitespace, fractional), and error cases (empty, invalid, negative, zero)
- Stub `RuntimeProvider` impl with `todo!()` bodies

Registered `pub mod docker` in lib.rs with `pub use docker::DockerProvider` re-export.

Created `crates/smelt-cli/tests/docker_lifecycle.rs` with 4 lifecycle tests (provision_and_teardown, exec, exec_nonzero_exit, teardown_on_error) plus helpers (`test_manifest()`, `assert_container_removed()`, `docker_provider_or_skip()`). Tests skip when Docker daemon is unavailable.

## Verification

- `cargo test -p smelt-core -- docker::tests` — 16 tests pass (resource parsing)
- `cargo build -p smelt-cli --tests` — integration test file compiles without errors
- `cargo test --workspace` — 93 total tests pass (74 smelt-core + 10 dry_run + 4 docker_lifecycle skip + 3 smelt-core unit + 2 doctests), zero failures, zero warnings (except pre-existing deprecation in dry_run.rs)
- Lifecycle tests correctly skip with "daemon not available" message when Docker socket not found

## Diagnostics

- `cargo test -p smelt-core -- docker::tests` — runs resource parsing unit tests
- `DockerProvider::new()` error includes operation="connect", bollard source error with socket path
- `client()` accessor on DockerProvider enables test code to call bollard inspect APIs directly

## Deviations

- Added `docker_provider_or_skip()` pattern to lifecycle tests so they skip gracefully when Docker isn't running, rather than panicking. Without this, `cargo test --workspace` would fail on machines without Docker. The task plan expected lifecycle tests to "fail at runtime due to todo!() stubs" but hard panics from missing Docker socket would mask that — the skip pattern is cleaner and the tests will properly hit todo!() panics once Docker is available.
- Used `bollard::query_parameters::InspectContainerOptions` instead of `bollard::container::InspectContainerOptions` — bollard 0.20 moved query parameter types to a separate module.

## Known Issues

None.

## Files Created/Modified

- `Cargo.toml` — added bollard and futures-util to workspace dependencies
- `crates/smelt-core/Cargo.toml` — added bollard and futures-util as dependencies
- `crates/smelt-cli/Cargo.toml` — added bollard and tokio to dev-dependencies
- `crates/smelt-core/src/docker.rs` — new: DockerProvider, resource parsers, RuntimeProvider stubs, 16 unit tests
- `crates/smelt-core/src/lib.rs` — registered docker module, added DockerProvider re-export
- `crates/smelt-cli/tests/docker_lifecycle.rs` — new: 4 lifecycle integration tests with skip-when-no-daemon pattern
