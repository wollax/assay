---
estimated_steps: 5
estimated_files: 4
---

# T01: Add bind-mount support to DockerProvider and validate repo path

**Slice:** S03 — Repo Mount & Assay Execution
**Milestone:** M001

## Description

Extend `DockerProvider::provision()` to bind-mount the host repo into the container at `/workspace`, and add repo path validation to reject URLs. The bind-mount uses bollard's `HostConfig.binds` field which is already partially constructed in `provision()`. Also extend `exec()` to set `working_dir: /workspace` so commands run in the repo context by default.

## Steps

1. Add `resolve_repo_path()` function to `manifest.rs` — takes `&str` (the `job.repo` value), rejects strings that look like URLs (starts with `http://`, `https://`, `git://`, `ssh://`, or contains `@` before `:`), canonicalizes to absolute path via `std::fs::canonicalize()`, returns `Result<PathBuf>`. Returns `SmeltError::Manifest` on failure.
2. Extend `DockerProvider::provision()` in `docker.rs` — call `resolve_repo_path(&manifest.job.repo)`, build the bind string as `"{resolved}:/workspace"`, add to `HostConfig { binds: Some(vec![bind_string]), ... }`. Add `tracing::info` for the resolved mount path.
3. Extend `DockerProvider::exec()` — add `working_dir: Some("/workspace".to_string())` to `CreateExecOptions`. This makes all exec commands run in the repo context.
4. Add unit tests in `manifest.rs` for `resolve_repo_path()`: valid absolute path, URL rejection (http, https, git, ssh), relative path resolution, nonexistent path error, path with spaces.
5. Add integration tests in `docker_lifecycle.rs`: `test_bind_mount_read` — create temp dir with a test file, build manifest with `job.repo` pointing to temp dir, provision container, exec `cat /workspace/test.txt` and verify content; `test_bind_mount_write` — exec `touch /workspace/newfile` and verify it exists on host; `test_repo_url_rejected` — verify manifest with URL repo produces validation error.

## Must-Haves

- [ ] `resolve_repo_path()` rejects URL-like strings with `SmeltError::Manifest`
- [ ] `resolve_repo_path()` canonicalizes relative paths to absolute
- [ ] `provision()` adds bind-mount string to `HostConfig.binds`
- [ ] `exec()` sets `working_dir` to `/workspace`
- [ ] Integration test verifies host file readable inside container at `/workspace`
- [ ] Integration test verifies URL repo path rejected before Docker interaction

## Verification

- `cargo test -p smelt-core -- manifest::tests::resolve_repo` — unit tests for path validation
- `cargo test -p smelt-cli --test docker_lifecycle -- mount` — bind-mount integration tests
- `cargo test --workspace` — no regressions

## Observability Impact

- Signals added/changed: `tracing::info` for resolved repo path and bind-mount configuration during provision
- How a future agent inspects this: `docker inspect <container>` shows the bind-mount in `Mounts`; `SMELT_LOG=info` shows the resolved path
- Failure state exposed: `SmeltError::Manifest` with the invalid repo path string included in the error message

## Inputs

- `crates/smelt-core/src/docker.rs` — existing `provision()` with `HostConfig` construction (from S02)
- `crates/smelt-core/src/manifest.rs` — existing `JobManifest` with `job.repo` string field (from S01)
- `crates/smelt-cli/tests/docker_lifecycle.rs` — existing test helpers: `docker_provider_or_skip()`, `test_manifest()`, `assert_container_removed()` (from S02)
- S02 forward intelligence: "To add bind-mounts, extend the `ContainerCreateBody` construction in provision() with `HostConfig.binds`"

## Expected Output

- `crates/smelt-core/src/manifest.rs` — new `resolve_repo_path()` public function with unit tests
- `crates/smelt-core/src/docker.rs` — `provision()` with bind-mount support, `exec()` with `working_dir`
- `crates/smelt-cli/tests/docker_lifecycle.rs` — new bind-mount integration tests
