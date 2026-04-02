# S03: Repo Mount & Assay Execution

**Goal:** `smelt run manifest.toml` provisions a container with the host repo bind-mounted and executes `assay run` (via a mock) inside it, streaming output to the terminal.
**Demo:** Run `smelt run` against a manifest pointing at a local repo path → container mounts the repo at `/workspace` → a mock assay script validates the mount and manifest → output streams to terminal → container tears down cleanly.

## Must-Haves

- `DockerProvider::provision()` bind-mounts `manifest.job.repo` (local absolute path) into the container at `/workspace`
- `manifest.job.repo` is validated as a local directory path; URLs are rejected with a clear error
- `AssayInvoker` translates `JobManifest` sessions into an Assay-compatible TOML manifest string
- The Assay manifest is written into the container via exec (base64-encode approach)
- `assay run` invocation is executed via `DockerProvider::exec()` with `working_dir: /workspace`
- `run.rs` orchestrates: provision (with mount) → write manifest → exec assay → stream output → teardown
- A mock script stands in for real `assay run` in tests, validating mount and manifest presence
- Exec commands use `working_dir: /workspace` so all commands run in repo context

## Proof Level

- This slice proves: integration (real Docker bind-mount + exec with mock assay)
- Real runtime required: yes (Docker daemon for integration tests)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-core -- assay::tests` — unit tests for manifest translation and repo path validation
- `cargo test -p smelt-cli --test docker_lifecycle -- mount` — integration tests verifying bind-mount read/write fidelity inside container
- `cargo test -p smelt-cli --test docker_lifecycle -- assay` — integration test verifying mock assay execution with mounted repo and written manifest
- `cargo test --workspace` — all existing + new tests pass, zero regressions
- Failure-path check: integration test verifies that a URL repo path produces `SmeltError::Manifest` with a clear message

## Observability / Diagnostics

- Runtime signals: `tracing::info` events for repo path resolution, bind-mount configuration, manifest file write, assay command construction and exit code
- Inspection surfaces: `docker inspect <container>` shows bind-mount in `Mounts` field; `SMELT_LOG=info smelt run` shows full lifecycle including mount and assay phases
- Failure visibility: `SmeltError::Manifest` for invalid repo path with the actual path in the message; `SmeltError::Provider` with exec context for assay failures; non-zero exit code from assay captured in `ExecHandle`
- Redaction constraints: credential env vars resolved at provision time (existing behavior) — assay manifest written to container may contain session specs but no secrets

## Integration Closure

- Upstream surfaces consumed: `DockerProvider` (provision/exec/teardown from S02), `JobManifest`/`SessionDef`/`MergeConfig` (from S01), `SmeltError` variants (from S01)
- New wiring introduced in this slice: `run.rs` now orchestrates mount + manifest-write + assay-exec instead of a health-check; `AssayInvoker` translates Smelt manifest to Assay format; `provision()` accepts bind-mount config
- What remains before the milestone is truly usable end-to-end: result collection from container (S04), timeout/signal handling (S05), end-to-end with real Assay and multi-session ordering (S06)

## Tasks

- [x] **T01: Add bind-mount support to DockerProvider and validate repo path** `est:40m`
  - Why: Containers need the host repo mounted at `/workspace` for Assay to operate on it. The `manifest.job.repo` field needs validation as a local path since bind-mounts require absolute host paths.
  - Files: `crates/smelt-core/src/docker.rs`, `crates/smelt-core/src/manifest.rs`, `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: Add `resolve_repo_path()` to manifest.rs that canonicalizes `job.repo` and rejects URLs. Extend `provision()` to populate `HostConfig.binds` with `resolved_path:/workspace`. Add `working_dir` to `CreateExecOptions` in `exec()`. Write integration tests: mount a temp dir, verify file read/write inside container, verify URL repo rejected.
  - Verify: `cargo test --workspace` passes; integration test creates file in temp dir on host, reads it from `/workspace` inside container via exec
  - Done when: `DockerProvider::provision()` bind-mounts the repo path and `exec()` runs commands in `/workspace`; URL repo paths produce a clear validation error

- [x] **T02: Create AssayInvoker with manifest translation and container file writing** `est:40m`
  - Why: Smelt must translate its session definitions into an Assay-compatible TOML manifest and deliver it into the container before invoking `assay run`. This is the core Assay integration boundary.
  - Files: `crates/smelt-core/src/assay.rs`, `crates/smelt-core/src/lib.rs`, `crates/smelt-core/src/error.rs`
  - Do: Create `assay.rs` with `AssayInvoker` struct. Implement `build_manifest_toml(&JobManifest) -> String` that maps Smelt sessions to Assay's `[[sessions]]` format. Implement `write_manifest(&DockerProvider, &ContainerId, &str) -> Result<()>` that base64-encodes the TOML and writes it via exec (`echo <b64> | base64 -d > /tmp/smelt-manifest.toml`). Implement `build_command(&JobManifest) -> Vec<String>` for the `assay run` CLI invocation. Add unit tests for manifest translation covering: single session, multi-session with depends_on, timeout mapping, special characters in spec text.
  - Verify: `cargo test -p smelt-core -- assay::tests` passes with manifest translation and command construction tests
  - Done when: `AssayInvoker` produces correct Assay-format TOML from Smelt sessions and constructs the right CLI command

- [x] **T03: Wire mount + assay invocation into CLI run and verify with mock** `est:45m`
  - Why: Closes the slice loop — `smelt run` must orchestrate the full flow: provision with mount → write assay manifest → exec assay → stream output → teardown. A mock script validates the integration without requiring real Assay.
  - Files: `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/tests/docker_lifecycle.rs`, `examples/job-manifest.toml`
  - Do: Replace health-check block in `execute_run()` with: resolve repo path → provision (mount included) → write assay manifest via `AssayInvoker` → exec assay command → check exit code → teardown. Create a mock assay integration test that: writes a shell script mock to container (`#!/bin/sh` that checks `/workspace` exists, reads `/tmp/smelt-manifest.toml`, prints session names), runs it via the assay exec path, asserts output contains expected session data. Update example manifest `job.repo` to use a placeholder local path. Update existing CLI lifecycle test to work with new flow.
  - Verify: `cargo test -p smelt-cli --test docker_lifecycle` passes all existing + new tests; `cargo test --workspace` green
  - Done when: `smelt run manifest.toml` with a local repo path mounts the repo, writes the assay manifest, executes the mock, streams output, and tears down — verified by integration test

## Files Likely Touched

- `crates/smelt-core/src/docker.rs` — bind-mount in provision, working_dir in exec
- `crates/smelt-core/src/manifest.rs` — repo path validation/resolution
- `crates/smelt-core/src/assay.rs` — new: AssayInvoker with manifest translation
- `crates/smelt-core/src/lib.rs` — register assay module
- `crates/smelt-core/src/error.rs` — possibly new error context for assay invocation
- `crates/smelt-cli/src/commands/run.rs` — orchestrate mount + assay flow
- `crates/smelt-cli/tests/docker_lifecycle.rs` — new integration tests for mount and assay mock
- `examples/job-manifest.toml` — update repo field for local path usage
