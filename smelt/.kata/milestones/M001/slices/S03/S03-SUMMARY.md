---
id: S03
parent: M001
milestone: M001
provides:
  - Bind-mount of host repo into container at /workspace with local path validation
  - AssayInvoker translating Smelt sessions into Assay-compatible TOML manifest
  - Base64-encoded manifest delivery into container via exec
  - Full orchestration flow in execute_run(): provision → write manifest → exec assay → teardown
  - Mock assay integration tests validating end-to-end path
requires:
  - slice: S02
    provides: DockerProvider with provision/exec/teardown, container lifecycle guarantees
  - slice: S01
    provides: JobManifest/SessionDef/MergeConfig types, SmeltError variants, RuntimeProvider trait
affects:
  - S04
  - S05
  - S06
key_files:
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-core/src/docker.rs
  - crates/smelt-core/src/assay.rs
  - crates/smelt-core/src/lib.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
  - examples/job-manifest.toml
key_decisions:
  - D027: Fixed /workspace mount point for host repo in container
  - D028: Base64-encode TOML manifest, write via exec to avoid heredoc quoting issues
  - D029: Smelt-side serde structs for Assay format (no crate dependency, per D002)
  - D030: Repo path validation — local paths only, URLs rejected with clear error
patterns_established:
  - resolve_repo_path() as validation gateway before bind-mount construction
  - AssayInvoker as stateless struct with associated functions for manifest translation
  - base64-encoded mock scripts written into containers for integration testing
  - Lifecycle phase messages on stderr for each orchestration step
observability_surfaces:
  - stderr lifecycle messages: Provisioning → Writing manifest → Executing assay run → Assay complete (exit code) → Tearing down → Container removed
  - tracing::info for resolved repo path, bind-mount string, manifest TOML generation, command construction
  - SmeltError::Manifest with field "job.repo" for invalid paths; SmeltError::Provider with operation "write_manifest" on delivery failures
  - Non-zero assay exit code surfaced with stderr content in error message
drill_down_paths:
  - .kata/milestones/M001/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S03/tasks/T02-SUMMARY.md
  - .kata/milestones/M001/slices/S03/tasks/T03-SUMMARY.md
duration: ~30m
verification_result: passed
completed_at: 2026-03-17
---

# S03: Repo Mount & Assay Execution

**Containers now bind-mount the host repo at `/workspace` and execute Assay via a translated TOML manifest — verified with mock scripts against real Docker.**

## What Happened

Three tasks built the repo-mount and Assay invocation pipeline:

**T01** added `resolve_repo_path()` to `manifest.rs` — it canonicalizes local paths and rejects URLs (http, https, git, ssh) and SCP-style syntax (`user@host:path`). `DockerProvider::provision()` now calls this and sets `HostConfig.binds` with `"{resolved}:/workspace"`. `exec()` sets `working_dir: /workspace` on all commands. 9 unit tests cover path validation; 4 integration tests verify bind-mount read/write fidelity and URL rejection.

**T02** created `AssayInvoker` in a new `assay.rs` module — a stateless translation layer with three functions: `build_manifest_toml()` maps Smelt `SessionDef` fields to Assay-format serde structs and serializes to TOML; `write_manifest_to_container()` base64-encodes the TOML and writes it to `/tmp/smelt-manifest.toml` via exec; `build_run_command()` constructs the `assay run` CLI invocation with the max session timeout. 6 unit tests verify manifest translation, command construction, special character handling, and round-trip TOML validity.

**T03** replaced the health-check in `execute_run()` with the full orchestration flow: resolve repo path → provision (with bind-mount) → write assay manifest → exec assay run → check exit code → teardown. Two new integration tests use mock shell scripts: one validates the mount is readable and the manifest contains expected sessions, the other verifies non-zero exit codes are captured with stderr content. The example manifest was updated to use a local path.

## Verification

- `cargo test -p smelt-core -- assay::tests` — 6/6 passed ✅
- `cargo test -p smelt-cli --test docker_lifecycle -- mount` — 3/3 passed ✅
- `cargo test -p smelt-cli --test docker_lifecycle -- assay` — 2/2 passed ✅
- `cargo test -p smelt-cli --test docker_lifecycle -- url_rejected` — 1/1 passed ✅
- `cargo test --workspace` — all tests passed, zero regressions ✅
- `cargo run -- run examples/job-manifest.toml --dry-run` — dry-run works with updated manifest ✅

## Deviations

- `test_manifest()` helper changed to use `"."` instead of a URL since `provision()` now validates repo paths. Added `test_manifest_with_repo()` for tests needing specific paths. This was necessary but not explicitly in the plan.
- `test_cli_run_lifecycle` updated to create a temp manifest with local repo instead of using `examples/job-manifest.toml`, and relaxed to not assert success (alpine lacks `assay` binary).

## Known Limitations

- `examples/job-manifest.toml` uses `"."` which works for `--dry-run` but `smelt run` without `--dry-run` requires an absolute path at runtime (canonicalization happens during provision)
- Assay manifest format is based on assumed contract (D029) — real Assay integration is deferred to S06
- No clone-into-container support (D030) — only local paths work for repo mounting
- No multi-repo mount support — single `/workspace` mount point (D027)

## Follow-ups

- S04 will consume the repo mount path (`/workspace`) to extract branch state after Assay completes
- S05 will consume exec output streams for progress monitoring and timeout enforcement
- S06 will exercise real `assay run` (not mock) and validate the manifest format assumption (D029)

## Files Created/Modified

- `crates/smelt-core/src/manifest.rs` — added `resolve_repo_path()` with URL rejection and canonicalization, 9 unit tests
- `crates/smelt-core/src/docker.rs` — `provision()` sets `HostConfig.binds`; `exec()` sets `working_dir: /workspace`
- `crates/smelt-core/src/assay.rs` — new: AssayInvoker with manifest translation, container writing, command construction, 6 unit tests
- `crates/smelt-core/src/lib.rs` — registered `pub mod assay` with re-export
- `crates/smelt-core/Cargo.toml` — added `base64.workspace = true`
- `Cargo.toml` — added `base64 = "0.22"` to workspace dependencies
- `crates/smelt-cli/src/commands/run.rs` — replaced health-check with full assay orchestration flow
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added 6 new tests (3 mount, 2 assay mock, 1 URL rejection); updated existing tests for local paths
- `crates/smelt-cli/Cargo.toml` — added `base64` dev-dependency
- `examples/job-manifest.toml` — changed repo to `"."` with local path guidance comments

## Forward Intelligence

### What the next slice should know
- The repo is mounted at `/workspace` inside the container. All exec commands run with `working_dir: /workspace`. S04's result collector should read branch state from this path before teardown.
- `AssayInvoker::write_manifest_to_container()` writes to `/tmp/smelt-manifest.toml`. If Assay produces output files, they'll be in `/workspace` (the mounted repo).
- `execute_run()` in `run.rs` is the orchestration hub — S04 adds result collection between "Assay complete" and teardown; S05 wraps the exec call with timeout/signal handling.

### What's fragile
- The Assay manifest format (D029) is assumed, not validated against real Assay — `AssaySession` serde structs may need adjustment when real Assay integration happens in S06.
- The base64 write approach (D028) assumes the container image has `base64` and `sh` — this holds for standard images but could break with minimal/distroless images.

### Authoritative diagnostics
- `docker inspect <container>` → `Mounts` field shows bind-mount configuration
- `SMELT_LOG=info smelt run` shows resolved repo path, bind string, manifest bytes, and assay command
- Integration tests in `docker_lifecycle.rs` are the ground truth for mount and assay behavior — they run against real Docker

### What assumptions changed
- No assumptions changed — the slice executed as planned. The Assay CLI contract (D002) remains untested with real Assay, as expected (deferred to S06).
