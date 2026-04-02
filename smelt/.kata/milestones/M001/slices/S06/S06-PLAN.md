# S06: End-to-End Integration

**Goal:** The full `smelt run` pipeline — provision → write manifest → exec assay → collect → teardown — works correctly end-to-end with real Docker, multi-session manifests, and both happy-path and error-path scenarios, verified by an integration test suite that chains all subsystems.
**Demo:** `cargo test -p smelt-cli --test docker_lifecycle` passes 100% with no pre-existing failures, including a new `test_full_e2e_pipeline` test that exercises every phase of the pipeline using a mock assay binary and confirms the result branch exists on the host after teardown.

## Must-Haves

- `test_collect_creates_target_branch` passes (alpine:3 has git installed before mock script runs)
- `test_cli_run_lifecycle` passes reliably (stale smelt containers pre-cleaned at test start)
- `test_full_e2e_pipeline` provisions a container, installs git, writes a mock assay binary at `/usr/local/bin/assay`, writes the smelt manifest, executes `assay run`, collects the result branch, tears down — asserts exit_code == 0, target branch exists, commit_count >= 1
- `test_multi_session_e2e` verifies a 2-session manifest (with `depends_on`) is serialized correctly into the container and mock assay receives it intact
- `test_e2e_assay_failure_no_orphans` verifies a mock assay that exits 1 causes no container orphans and the run completes without hanging
- All 132 existing tests continue to pass (zero regressions)

## Proof Level

- This slice proves: final-assembly (integration)
- Real runtime required: yes — all Docker tests require a running Docker daemon; they skip gracefully via `docker_provider_or_skip()` when unavailable
- Human/UAT required: no — integration tests provide full coverage

## Verification

- `cargo test -p smelt-cli --test docker_lifecycle 2>&1 | tail -5` — all tests pass, zero failures
- `cargo test --workspace 2>&1 | tail -5` — full workspace still clean
- `cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline` — pipeline test passes, result branch confirmed present
- `cargo test -p smelt-cli --test docker_lifecycle -- test_e2e_assay_failure_no_orphans` — failure path test passes, container absence confirmed

## Observability / Diagnostics

- Runtime signals: phase transition messages on stderr (`Provisioning...`, `Writing manifest...`, `Executing assay run...`, `Collecting results...`, `Container removed.`) — already in `run_with_cancellation()`; E2E test asserts these strings in subprocess stderr
- Inspection surfaces: `.smelt/run-state.toml` written by `JobMonitor` — E2E test for the failure path verifies the state file reflects `Failed` phase or is cleaned up (absence of stale state is itself a signal)
- Failure visibility: non-zero exit_code from mock assay propagates as `anyhow::bail!` in `execute_run()`; container removal verified via `docker ps --filter label=smelt.job -q` returning empty
- Redaction constraints: none — test environment only; no real credentials in test manifests

## Integration Closure

- Upstream surfaces consumed: `DockerProvider` (S02), `AssayInvoker::write_manifest_to_container` + `build_run_command` (S03), `ResultCollector::collect` (S04), `run_with_cancellation` + `JobMonitor` (S05)
- New wiring introduced in this slice: integration test `test_full_e2e_pipeline` is the first test to chain all 8 phases end-to-end; `test_multi_session_e2e` verifies multi-session manifest serialization; `test_e2e_assay_failure_no_orphans` closes the error path
- What remains before the milestone is truly usable end-to-end: nothing — all subsystems are wired and tested; M001 milestone definition of done is satisfied

## Tasks

- [x] **T01: Fix two pre-existing Docker integration test failures** `est:15m`
  - Why: `test_collect_creates_target_branch` and `test_cli_run_lifecycle` have known failures that make the baseline noisy and mask regressions; must be fixed before adding new tests
  - Files: `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: (1) In `test_collect_creates_target_branch`: add `provider.exec(&container, &["sh", "-c", "apk add --no-cache git"]).await?` immediately after provisioning the container and before writing the mock script. (2) In `test_cli_run_lifecycle`: add a pre-clean block at test start that runs `docker ps -aq --filter label=smelt.job`, splits stdout by whitespace, and calls `docker rm -f <id>` for each non-empty ID. Use `std::process::Command` for both docker calls. The pre-clean is defensive — tolerate empty output silently.
  - Verify: `cargo test -p smelt-cli --test docker_lifecycle -- collect` passes; `cargo test -p smelt-cli --test docker_lifecycle -- cli_run_lifecycle` passes
  - Done when: both named tests show `ok` with no pre-existing failure messages

- [x] **T02: Add full end-to-end pipeline integration test** `est:25m`
  - Why: No existing test chains all phases (provision → install git → write mock assay → write manifest → exec assay → collect → teardown); this is the core S06 deliverable
  - Files: `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: Add `test_full_e2e_pipeline` as an `#[tokio::test]` after the existing collect test. Steps: (1) create temp git repo with initial commit using existing `setup_test_repo`-style inline helpers (init, config user.email/name, write README, add, commit); (2) record `base_ref = git rev-parse HEAD`; (3) build manifest via `test_manifest_with_repo("e2e-pipeline", repo_dir)` with `manifest.job.base_ref = base_ref` and `manifest.merge.target = "smelt/e2e-result"`; (4) provision container; (5) install git via `provider.exec(&container, &["sh", "-c", "apk add --no-cache git"])` — assert exit_code == 0; (6) write mock assay binary to `/usr/local/bin/assay` via base64-encoded shell script — script must: `cd /workspace`, `git config user.email/name`, `echo "generated" > assay-output.txt`, `git add assay-output.txt`, `git commit -m "assay: generated output"`, `exit 0`; (7) write smelt manifest via `AssayInvoker::write_manifest_to_container`; (8) exec assay via `provider.exec(&container, &AssayInvoker::build_run_command(&manifest))` — assert exit_code == 0; (9) collect via `ResultCollector::new(git_cli, repo_path).collect(&base_ref, "smelt/e2e-result")`; (10) teardown and assert container removed; (11) assert `!result.no_changes`, `result.commit_count >= 1`, target branch exists on host (via `git rev-parse --verify smelt/e2e-result`), `result.files_changed.contains("assay-output.txt")`.
  - Verify: `cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline` shows `ok`
  - Done when: test passes end-to-end with all 11 assertions satisfied and no container leak

- [x] **T03: Add multi-session and error-path integration tests** `est:20m`
  - Why: Completes the S06 must-haves — multi-session manifest serialization and the error path (assay failure → teardown, no orphans) are not yet covered
  - Files: `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: (1) Add `test_multi_session_e2e`: build a manifest with two `SessionDef` entries where the second has `depends_on: ["session-one"]`; provision container; install git; write mock assay (reads `/tmp/smelt-manifest.toml`, verifies it exists, creates a commit, exits 0); write manifest via `AssayInvoker::write_manifest_to_container`; exec a verification step via `provider.exec(&container, &["sh", "-c", "cat /tmp/smelt-manifest.toml"])` and assert stdout contains both session names; exec assay via `build_run_command`; assert exit_code == 0; teardown. (2) Add `test_e2e_assay_failure_no_orphans`: provision container; write a mock assay script to `/usr/local/bin/assay` that immediately `exit 1`; write smelt manifest; exec `provider.exec(&container, &AssayInvoker::build_run_command(&manifest))`; assert exit_code == 1; call `provider.teardown(&container)`; assert container removed via `assert_container_removed`; call `docker ps -aq --filter label=smelt.job` and assert empty.
  - Verify: `cargo test -p smelt-cli --test docker_lifecycle -- multi_session` passes; `cargo test -p smelt-cli --test docker_lifecycle -- failure_no_orphans` passes; `cargo test --workspace` — full suite clean
  - Done when: both tests pass, full `cargo test --workspace` shows zero failures

## Files Likely Touched

- `crates/smelt-cli/tests/docker_lifecycle.rs`
