# S06: End-to-End Integration — UAT

**Milestone:** M001
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All S06 verification is exercised by automated integration tests running against a real Docker daemon. The tests provision real containers, execute real exec calls, and inspect real filesystem/git state. No human interaction is required to confirm correct behavior — the test output is the proof.

## Preconditions

- Docker daemon is running and reachable (`docker info` returns without error)
- Host has network access to `dl-cdn.alpinelinux.org` (for `apk add --no-cache git` inside containers)
- `cargo test` is available and the workspace builds cleanly
- No pre-existing smelt containers (`docker ps -aq --filter label=smelt.job` returns empty, or tests will pre-clean them)

## Smoke Test

```
cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline
```

Expected: `test test_full_e2e_pipeline ... ok` — confirms the entire pipeline chain (provision → git install → mock assay → manifest write → exec → collect → teardown) runs without error.

## Test Cases

### 1. Full E2E Pipeline — Happy Path

```
cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline --nocapture
```

1. Test creates a temp git repo with an initial commit
2. Provisions an alpine:3 container, installs git
3. Writes mock assay binary to `/usr/local/bin/assay`
4. Writes smelt manifest via `AssayInvoker::write_manifest_to_container()`
5. Executes `assay run /tmp/smelt-manifest.toml --timeout 60` inside container
6. Calls `ResultCollector::collect()` on host repo
7. Tears down container and verifies removal
8. **Expected:** exit_code == 0, `!result.no_changes`, `result.commit_count >= 1`, `result.files_changed.contains("assay-output.txt")`, `git rev-parse --verify smelt/e2e-result` succeeds on host, no container remains

### 2. Multi-Session Manifest Round-Trip

```
cargo test -p smelt-cli --test docker_lifecycle -- test_multi_session_e2e --nocapture
```

1. Builds 2-session manifest with `session-two` depending on `session-one`
2. Provisions container, installs git
3. Writes manifest via `AssayInvoker::write_manifest_to_container()`
4. Reads back `/tmp/smelt-manifest.toml` via `provider.exec()`
5. Executes mock assay that verifies manifest exists then exits 0
6. **Expected:** serialized TOML contains `session-one`, `session-two`, and `depends_on = ["session-one"]`; assay exec exits 0; container torn down cleanly

### 3. Assay Failure — No Container Orphans

```
cargo test -p smelt-cli --test docker_lifecycle -- test_e2e_assay_failure_no_orphans --nocapture
```

1. Provisions container
2. Writes mock assay binary that immediately `exit 1`
3. Executes assay via `provider.exec()`
4. Calls `provider.teardown()`
5. Checks `docker ps --filter label=smelt.job=failure-no-orphans -q`
6. **Expected:** exec exits with code 1; teardown returns `Ok(())`; bollard inspect returns 404; docker ps returns empty output

## Edge Cases

### Docker Daemon Unavailable

```
# With Docker stopped, run:
cargo test -p smelt-cli --test docker_lifecycle
```

**Expected:** All Docker-dependent tests skip via `docker_provider_or_skip!()` macro; test suite exits 0 with skip counts reported.

### Pre-existing Orphan Containers

1. Manually create a labeled container: `docker run -d --label smelt.job=test-job alpine sleep 3600`
2. Run: `cargo test -p smelt-cli --test docker_lifecycle -- test_cli_run_lifecycle`
3. **Expected:** Pre-clean block removes the orphan container; test proceeds without failing on the pre-existing container

### Full Workspace

```
cargo test --workspace 2>&1 | grep "^test result"
```

**Expected:** `docker_lifecycle` and `smelt-core` suites show `0 failed`. The pre-existing `run_without_dry_run_attempts_docker` failure in `dry_run` is known and unrelated to S06.

## Failure Signals

- `apk add --no-cache git` exits non-zero → network unreachable in CI; use a pre-built image with git
- `assay run` exec exits non-zero in happy-path test → mock binary not on PATH or not executable; check `/usr/local/bin/assay` via `provider.exec(&container, &["which", "assay"])`
- `ResultCollector::collect()` returns error → confirm bind-mount is writable and git identity was configured inside container
- `docker ps -a --filter label=smelt.job -q` returns non-empty after tests → container leak; teardown is not being called on all paths
- `cargo test --workspace` shows new failures beyond `run_without_dry_run_attempts_docker` → regression introduced; bisect by running `docker_lifecycle` suite in isolation first

## Requirements Proved By This UAT

No `.kata/REQUIREMENTS.md` exists. In lieu of formal requirement IDs, the following M001 success criteria are proved:

- ✅ Multi-session manifest with dependencies is serialized correctly and survives the container boundary intact
- ✅ The full deploy → execute → collect → teardown cycle completes without manual intervention for the happy path
- ✅ Container failures (non-zero assay exit) are detected, teardown is called, and no container orphans remain
- ✅ Result branch exists on host after job completion with expected commits
- ✅ `smelt run --dry-run` is unaffected (covered in existing dry_run suite, untouched by S06)

## Not Proven By This UAT

- `smelt run manifest.toml` through the real CLI entrypoint (tests bypass `run_with_cancellation()` to inject mock assay setup — D039)
- `smelt status` live progress display during a real run (covered in S05 monitor tests)
- Credential injection end-to-end (unit-tested in S01/S02; no integration test with real secrets)
- Timeout enforcement at the `run_with_cancellation()` level (covered by S05 timeout tests, not re-exercised in S06)
- Multi-session dependency *execution ordering* — the test confirms serialization fidelity but does not verify Assay executes sessions in topological order (Assay's responsibility, not Smelt's)
- Container OOM / crash recovery (Docker resource limit integration not tested)

## Notes for Tester

- All Docker tests skip gracefully when the daemon is unavailable — a "skipped" result is correct behavior in environments without Docker, not a failure.
- The `run_without_dry_run_attempts_docker` failure in `dry_run.rs` is pre-existing and should be ignored when evaluating S06.
- Tests that install alpine packages take ~2–4s longer than tests that don't — this is expected; `apk add --no-cache git` downloads ~8MB.
- Run with `--nocapture` to see full exec output for debugging: `cargo test -p smelt-cli --test docker_lifecycle -- test_full_e2e_pipeline --nocapture`
