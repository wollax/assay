---
id: T01
parent: S03
milestone: M001
provides:
  - resolve_repo_path() function for local path validation
  - Bind-mount of host repo into container at /workspace
  - working_dir set to /workspace for exec commands
key_files:
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-core/src/docker.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - SCP-style SSH syntax (user@host:path) rejected via @-before-: heuristic
  - test_manifest() helper updated to use local path "." instead of URL — all existing tests adapted
  - CLI lifecycle test now creates a temp manifest with local repo instead of using examples/job-manifest.toml
patterns_established:
  - resolve_repo_path() as the validation gateway before any bind-mount construction
  - test_manifest_with_repo() helper for integration tests needing specific repo paths
observability_surfaces:
  - tracing::info for resolved repo path and bind-mount string during provision
  - SmeltError::Manifest with field "job.repo" and the invalid path in the message for URL/nonexistent rejections
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add bind-mount support to DockerProvider and validate repo path

**Added `resolve_repo_path()` validation, bind-mount in `provision()`, and `working_dir` in `exec()` — verified with 9 unit tests and 4 integration tests.**

## What Happened

1. Added `resolve_repo_path()` to `manifest.rs` — rejects URL prefixes (http, https, git, ssh) and SCP-style SSH syntax (`user@host:path`), canonicalizes to absolute path via `std::fs::canonicalize()`.
2. Extended `DockerProvider::provision()` to call `resolve_repo_path(&manifest.job.repo)`, build the bind string `"{resolved}:/workspace"`, and set it on `HostConfig.binds`.
3. Extended `DockerProvider::exec()` to set `working_dir: Some("/workspace".to_string())` on `CreateExecOptions`.
4. Added 9 unit tests for `resolve_repo_path()`: valid absolute, relative, URL rejections (http/https/git/ssh/scp), nonexistent path, path with spaces.
5. Added 4 integration tests: `test_bind_mount_read` (host file readable in container), `test_bind_mount_write` (container writes visible on host), `test_bind_mount_working_dir` (pwd = /workspace, relative file access works), `test_repo_url_rejected` (URL produces SmeltError::Manifest).
6. Updated `test_manifest()` helper and CLI lifecycle test to use local paths instead of URLs (required since `provision()` now validates repo path).

## Verification

- `cargo test -p smelt-core -- manifest::tests::resolve_repo` — 9/9 passed
- `cargo test -p smelt-cli --test docker_lifecycle -- mount` — 3/3 passed
- `cargo test -p smelt-cli --test docker_lifecycle -- url_rejected` — 1/1 passed
- `cargo test --workspace` — 107/107 passed, zero regressions

Slice-level checks status:
- `cargo test -p smelt-core -- assay::tests` — not yet applicable (T02 scope)
- `cargo test -p smelt-cli --test docker_lifecycle -- mount` — ✅ passed
- `cargo test -p smelt-cli --test docker_lifecycle -- assay` — not yet applicable (T03 scope)
- `cargo test --workspace` — ✅ passed
- Failure-path check (URL rejection) — ✅ passed

## Diagnostics

- `SMELT_LOG=info smelt run <manifest>` shows resolved repo path and bind string
- `docker inspect <container>` shows bind-mount in `Mounts` array
- Invalid repo paths produce `SmeltError::Manifest { field: "job.repo", message: "repo must be a local path, not a URL: ..." }`
- Nonexistent paths produce `SmeltError::Manifest { field: "job.repo", message: "cannot resolve repo path ...: No such file or directory" }`

## Deviations

- Updated `test_manifest()` helper to use `"."` (cwd) instead of a URL since `provision()` now rejects URLs. Added `test_manifest_with_repo()` for tests needing specific repo paths.
- Updated `test_cli_run_lifecycle` to create a temp manifest with local repo path instead of using `examples/job-manifest.toml` (which still has a URL for documentation purposes).

## Known Issues

- `examples/job-manifest.toml` still uses a URL for `job.repo` — this is fine for `--dry-run` (which doesn't call `provision()`) but will fail with `smelt run` without `--dry-run`. T03 will update the example.

## Files Created/Modified

- `crates/smelt-core/src/manifest.rs` — added `resolve_repo_path()` public function with URL rejection and canonicalization, plus 9 unit tests
- `crates/smelt-core/src/docker.rs` — `provision()` calls `resolve_repo_path()` and sets `HostConfig.binds`; `exec()` sets `working_dir: /workspace`
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added `test_manifest_with_repo()` helper, 4 new integration tests (bind_mount_read, bind_mount_write, bind_mount_working_dir, repo_url_rejected), updated CLI lifecycle test for local path
