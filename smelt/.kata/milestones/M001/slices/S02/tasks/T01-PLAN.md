---
estimated_steps: 5
estimated_files: 7
---

# T01: Add bollard dependency, resource parser, and integration test scaffolding

**Slice:** S02 — Docker Container Provisioning & Teardown
**Milestone:** M001

## Description

Establishes the foundation for S02: adds bollard and futures-util as workspace dependencies, creates the `docker` module with `DockerProvider` struct and resource string parsing utilities, and writes integration test scaffolding that defines the acceptance criteria for the slice. Resource parsing tests pass immediately; lifecycle integration tests compile but fail (methods are unimplemented stubs).

## Steps

1. Add `bollard = "0.20"` and `futures-util = "0.3"` to workspace `[workspace.dependencies]` in root `Cargo.toml`. Add both as dependencies of `smelt-core`. Add `tokio` and `bollard` to `smelt-cli` `[dev-dependencies]` for async integration tests.
2. Create `crates/smelt-core/src/docker.rs` with:
   - `DockerProvider` struct holding `bollard::Docker` client
   - `DockerProvider::new()` → connects via `Docker::connect_with_socket_defaults()`, returns `Result<Self>` wrapping bollard errors
   - `parse_memory_bytes(s: &str) -> Result<i64>` — handles "4G", "512M", "1024K", plain bytes
   - `parse_cpu_nanocpus(s: &str) -> Result<i64>` — handles "2", "0.5", etc. (multiply by 1_000_000_000)
   - Unit tests for both parsers covering valid inputs and error cases
   - Stub `RuntimeProvider` impl with `todo!()` bodies for `provision`, `exec`, `collect`, `teardown`
3. Register `pub mod docker;` in `crates/smelt-core/src/lib.rs` and add `pub use docker::DockerProvider;` to re-exports.
4. Create `crates/smelt-cli/tests/docker_lifecycle.rs` with integration tests:
   - `test_provision_and_teardown` — provisions a container from alpine:3, verifies it exists, tears down, verifies it's gone
   - `test_exec` — provisions, execs `echo hello`, checks output contains "hello" and exit code 0, tears down
   - `test_exec_nonzero_exit` — provisions, execs `sh -c "exit 42"`, checks exit code 42, tears down
   - `test_teardown_on_error` — provisions, attempts an operation that fails, verifies teardown still cleans up
   - All tests should be `#[tokio::test]` and use a helper function to build a minimal `JobManifest` for testing
5. Verify: `cargo test -p smelt-core -- docker::tests` passes (resource parsing). `cargo build -p smelt-cli --tests` compiles the integration test file. Lifecycle tests will fail at runtime due to `todo!()` stubs — that's expected and correct.

## Must-Haves

- [ ] `bollard` and `futures-util` in workspace deps and smelt-core deps
- [ ] `DockerProvider` struct with `new()` connecting to Docker socket
- [ ] `parse_memory_bytes` and `parse_cpu_nanocpus` with unit tests
- [ ] Integration test file with lifecycle test cases that compile
- [ ] `docker` module registered in `lib.rs` with `DockerProvider` re-exported

## Verification

- `cargo test -p smelt-core -- docker::tests` — resource parsing unit tests pass
- `cargo build -p smelt-cli --tests` — integration tests compile without errors
- `cargo test --workspace` — existing 71 tests still pass (no regressions)

## Observability Impact

- Signals added/changed: None yet (stubs only)
- How a future agent inspects this: `cargo test -p smelt-core -- docker::tests` runs resource parsing tests
- Failure state exposed: `SmeltError::Provider` wraps bollard connection errors in `DockerProvider::new()`

## Inputs

- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait definition, `ContainerId`, `ExecHandle`, `CollectResult`
- `crates/smelt-core/src/manifest.rs` — `JobManifest` type for test fixture construction
- `crates/smelt-core/src/error.rs` — `SmeltError::provider()` and `SmeltError::provider_with_source()` constructors
- S02-RESEARCH.md — bollard API patterns, `ContainerCreateBody`, builder patterns, exec attach semantics

## Expected Output

- `Cargo.toml` — bollard + futures-util in workspace deps
- `crates/smelt-core/Cargo.toml` — bollard + futures-util in deps
- `crates/smelt-core/src/docker.rs` — DockerProvider struct, resource parsers with tests, RuntimeProvider stub impl
- `crates/smelt-core/src/lib.rs` — docker module registered
- `crates/smelt-cli/Cargo.toml` — tokio + bollard in dev-deps
- `crates/smelt-cli/tests/docker_lifecycle.rs` — integration test scaffolding (compiles, lifecycle tests fail at todo!())
