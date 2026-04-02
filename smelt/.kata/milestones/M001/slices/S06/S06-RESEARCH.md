# S06: End-to-End Integration — Research

**Date:** 2026-03-17

## Summary

S06 closes M001 by exercising the full `smelt run` pipeline as an integrated system. All individual subsystems are already verified by prior slices; the remaining work is (1) fix two pre-existing test failures that S05 identified, (2) add an end-to-end test that exercises every phase in sequence using a mock assay binary, and (3) verify multi-session manifests and error paths through the complete pipeline.

The pipeline is already functionally complete. `execute_run()` / `run_with_cancellation()` in `run.rs` chains all 8+ phases: provision → write manifest → exec assay → collect → teardown, with `JobMonitor` tracking every phase transition and `tokio::select!` enforcing timeout and Ctrl+C cancellation. What's missing is a comprehensive integration test that covers the happy path end-to-end (including a mock assay that actually creates commits so collection succeeds), plus the two pre-existing failures that make the current test suite noisy.

S06 has no new library dependencies, no new architectural decisions, and no new modules. The entire slice is integration-test work on top of the existing codebase.

## Recommendation

Write integration tests that chain the subsystem APIs directly (provider → assay → collector), using a mock assay script installed into the container before the main exec. Do **not** try to test through `run_with_cancellation()` for the full E2E test — that function provisions the container internally, making it impossible to inject the mock assay setup step. Instead, follow the same pattern established by `test_collect_creates_target_branch`: manually chain the phases in the test body.

Fix the two pre-existing failures as discrete tasks before adding new tests.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Container cleanup before test | `docker ps --filter label=smelt.job -q` + `docker rm -f` | Already how `test_cli_run_lifecycle` verifies cleanup; invert it to pre-clean |
| Install git in alpine for test | `apk add --no-cache git` via `provider.exec()` | Verified: adds git in ~2s, produces `git version 2.52.0` |
| Mock assay binary | Shell script written via `AssayInvoker::write_manifest_to_container()` pattern | Already established by `test_assay_mock_execution` and `test_collect_creates_target_branch` |
| Repo setup for collection test | `setup_test_repo()` + `add_commit()` + `head_hash()` helpers | Already in `collector::tests` — copy pattern into integration test |

## Existing Code and Patterns

- `crates/smelt-cli/src/commands/run.rs` — `run_with_cancellation<F>()` is the full pipeline; phases are: load → validate → runtime-check → monitor-init → provision → write-manifest → select!(exec+collect vs timeout vs cancel) → teardown. **Cannot inject setup steps between provision and exec when using this function.** Direct phase-chaining is the integration test pattern.
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `test_collect_creates_target_branch` is the closest existing analog to the full E2E test: init git repo → provision → mock script creates commits → collect → assert branch. S06 extends this by also exercising the assay exec phase and monitor state.
- `crates/smelt-cli/tests/docker_lifecycle.rs::test_cli_run_lifecycle` — Uses `assert_cmd::Command::cargo_bin("smelt")` through the real binary. Leaks containers when previous test runs didn't clean up (stale containers labeled `smelt.job`). The final assertion `remaining.trim().is_empty()` fails because containers from concurrent or prior runs remain. **Fix: sweep `docker rm -f $(docker ps -aq --filter label=smelt.job)` before the assertion, or pre-clean at test start.**
- `crates/smelt-cli/tests/docker_lifecycle.rs::test_collect_creates_target_branch` — Fails because the mock assay script runs `git` inside alpine:3, which lacks git. **Fix: add `apk add --no-cache git` as the first exec step in the test setup, before the mock script runs.**
- `crates/smelt-core/src/assay.rs` — `AssayInvoker::build_run_command()` always produces `["assay", "run", "/tmp/smelt-manifest.toml", "--timeout", "<max>"]`. The mock assay binary must be at `assay` on PATH (`/usr/local/bin/assay`) inside the container.
- `crates/smelt-core/src/monitor.rs` — `JobMonitor::read()` reads `.smelt/run-state.toml`. E2E tests that go through `run_with_cancellation()` can assert the state file is cleaned up after successful completion.
- `crates/smelt-cli/src/lib.rs` — `pub mod commands` exposes `run_with_cancellation()` to integration tests. Import as `use smelt_cli::commands::run::run_with_cancellation`.

## Constraints

- D012 (no Dockerfile building): all test containers use `alpine:3` with packages installed via exec. No custom images.
- D021 (sleep 3600 keep-alive): containers stay running; all setup is via exec. The mock assay binary must be written into the container before `run_with_cancellation()` or the assay exec step.
- D024 (Docker test skip pattern): all new tests must call `docker_provider_or_skip()` and return early if Docker is unavailable.
- D029 (Smelt-side serde for Assay format): the manifest written to the container is Smelt's `AssayManifest` TOML, not Assay's native format — but for S06 this is fine because the mock assay doesn't care about format.
- `run_with_cancellation<F>()` cannot be used for the full E2E test because it provisions internally. The pattern is to manually chain phases in the test (provision → setup → write manifest → exec → collect → teardown).

## Common Pitfalls

- **`test_cli_run_lifecycle` container leak** — Parallel test execution creates multiple smelt containers; when any test run fails between provision and teardown, containers accumulate with the `smelt.job` label. Pre-clean at test start via `docker ps -aq --filter label=smelt.job | xargs docker rm -f` (tolerating empty output). The existing assertion checks `remaining.trim().is_empty()` — it will be satisfied after the run completes if smelt's own teardown works, but pre-existing orphans from prior runs cause spurious failures.
- **`test_collect_creates_target_branch` alpine lacks git** — The mock script does `git config` and `git commit` inside the container. `alpine:3` has no git. Add `provider.exec(&container, &["sh", "-c", "apk add --no-cache git"])` before running the mock script. This adds ~2s but is the simplest fix without changing the base image.
- **Monitor state_dir in E2E test** — `run_with_cancellation()` derives `state_dir` from `args.manifest.parent().join(".smelt")`. If using a tempdir for the manifest, the state dir will be inside that tempdir — not in the cwd. Verify the state file is cleaned up by checking `!state_dir.join("run-state.toml").exists()`.
- **Parallel Docker tests** — The docker_lifecycle tests currently run sequentially within the integration test binary but may conflict with other test binaries. Use unique job names per test (already done with distinct names like `"collect-test"`, `"timeout-teardown"`).
- **`exec_future` captures `monitor` mutably** — Inside `run_with_cancellation()`, the exec async block captures `monitor` by mutable reference. The E2E test does not need to replicate this complexity — it can ignore the monitor entirely or check the state file after completion.
- **Mock assay must exit 0** — `execute_run()` checks `handle.exit_code != 0` and propagates an error before collection runs. The mock assay script must `exit 0`.

## Open Risks

- **Pre-existing container orphans**: 6 smelt-labeled containers are currently present on the Docker daemon (`docker ps -a --filter "label=smelt.job" -q` returns 6 IDs). These will cause `test_cli_run_lifecycle` to fail immediately. The pre-clean sweep is mandatory, not optional.
- **`test_cli_run_lifecycle` non-deterministic**: This test runs the real `smelt` binary against an alpine container without assay. Smelt will provision, write manifest, exec `assay` (exit 127), then teardown. The test only checks lifecycle messages, not exit code. This should be reliable once the container orphan issue is resolved.
- **Git user config inside container**: The `test_collect_creates_target_branch` mock script runs `git config user.email` and `git config user.name` inside the container. After installing git via apk, these configs need to be set before committing — the existing script already does this.
- **Collection with `base_ref` as full hash vs branch name**: `execute_run()` uses `manifest.job.base_ref` as the base for collection. In integration tests, `base_ref` is set to the full commit hash (HEAD before provisioning). This works correctly with `GitCli::rev_parse()` — no risk here.

## Implementation Plan

### T01: Fix pre-existing test failures (~10 min)

**`test_collect_creates_target_branch`** (line ~592 in docker_lifecycle.rs):
- Add `provider.exec(&container, &["sh", "-c", "apk add --no-cache git"]).await?` before the mock script write step
- The mock script already sets git user config; no other changes needed

**`test_cli_run_lifecycle`** (line ~210 in docker_lifecycle.rs):
- At test start, add a pre-clean step: run `docker ps -aq --filter label=smelt.job` and `docker rm -f` on any returned IDs
- Use `std::process::Command::new("docker").args(["ps", "-aq", "--filter", "label=smelt.job"])` + loop through output calling `docker rm -f <id>`

### T02: Full end-to-end integration test (~20 min)

Add `test_full_e2e_pipeline` in `docker_lifecycle.rs`:

```
1. Init temp git repo with initial commit (reuse setup_test_repo pattern)
2. Record base_ref (initial HEAD hash)
3. Provision container with bind-mount to the repo
4. Install git in container: apk add --no-cache git
5. Write mock assay binary to /usr/local/bin/assay:
   - Script reads /tmp/smelt-manifest.toml (verify it exists)
   - Creates file + commit in /workspace
   - Exits 0
6. Write smelt manifest via AssayInvoker::write_manifest_to_container()
7. Exec assay via provider.exec(&container, &AssayInvoker::build_run_command(&manifest))
8. Assert handle.exit_code == 0
9. Collect via ResultCollector::collect(&base_ref, &target_branch)
10. Teardown
11. Assert: !no_changes, commit_count >= 1, target branch exists on host
```

This exercises: provisioning, assay manifest serialization, exec, collection, teardown — the full deploy → execute → collect → teardown cycle.

### T03: Multi-session manifest verification (~10 min)

Add `test_multi_session_e2e` that uses a manifest with 2+ sessions (with `depends_on`) and verifies:
- The manifest written to the container contains all sessions with dependency info intact
- The assay mock receives the full manifest and exits 0
- Collection succeeds after mock assay creates commits

This doesn't test that Assay actually respects dependency ordering (that's Assay's concern per D002), but it verifies that Smelt correctly passes multi-session manifests through.

### T04: Error path verification (~10 min)

Add `test_e2e_assay_failure_no_orphans` that uses a mock assay that exits 1, then verifies:
- Container is torn down (no orphans)
- The smelt run completes without hanging
- Monitor state cleanup (optional: check `.smelt/run-state.toml` is removed or has `Failed` phase)

This exercises the error path in `execute_run()`: assay non-zero exit → `Err(anyhow::bail!)` → `ExecOutcome::Completed(Err)` → `JobPhase::Failed` → teardown → cleanup.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | none needed | none — pure Rust test code |
| Docker/bollard | none | none — patterns already established in codebase |

## Sources

- S04 summary `Forward Intelligence`: collection runs inside the async block, teardown always follows — no change needed for S06
- S05 summary `Follow-ups`: "S06 should address the pre-existing `test_cli_run_lifecycle` container leak" and "S06 should consider whether `run_with_cancellation()` should be called through the full `smelt run` entrypoint in integration tests once assay mock is available"
- Direct verification: `docker run --rm alpine:3 sh -c "apk add --no-cache git && git --version"` → `git version 2.52.0` (works, ~2s)
- Direct verification: `docker ps -a --filter "label=smelt.job" -q` → 6 orphaned containers currently on daemon
- `cargo test --workspace --lib` → 105 unit tests pass; 2 Docker integration test failures in docker_lifecycle.rs
