---
estimated_steps: 4
estimated_files: 1
---

# T01: Fix two pre-existing Docker integration test failures

**Slice:** S06 — End-to-End Integration
**Milestone:** M001

## Description

Two Docker integration tests in `docker_lifecycle.rs` have known failures that pre-date S06:

1. **`test_collect_creates_target_branch`** — The mock assay script runs git commands inside `alpine:3`, but alpine does not ship with git. The fix is to install git via `apk add --no-cache git` in a `provider.exec()` call immediately after provisioning the container, before the mock script is written or run.

2. **`test_cli_run_lifecycle`** — The test asserts no containers remain after `smelt run` completes, but stale containers from prior (failed) test runs accumulate with the `smelt.job` label. The fix is to pre-clean all smelt-labeled containers at test start using `docker ps -aq --filter label=smelt.job` + `docker rm -f`.

Both fixes are surgical — no new logic, no new files, no architectural changes.

## Steps

1. Open `crates/smelt-cli/tests/docker_lifecycle.rs` and locate `test_collect_creates_target_branch` (currently at line ~593).

2. After the `provider.provision(&manifest).await` call and before the mock script write step, insert:
   ```rust
   // Install git in alpine (not present by default)
   let install = provider
       .exec(&container, &["sh", "-c", "apk add --no-cache git"])
       .await
       .expect("install git");
   assert_eq!(install.exit_code, 0, "git install should succeed");
   ```

3. Locate `test_cli_run_lifecycle` (currently at line ~220). At the very top of the test body (after the `docker_provider_or_skip()` guard), insert a pre-clean block:
   ```rust
   // Pre-clean any stale smelt containers from prior test runs
   let stale = std::process::Command::new("docker")
       .args(["ps", "-aq", "--filter", "label=smelt.job"])
       .output()
       .expect("docker ps");
   for id in String::from_utf8_lossy(&stale.stdout).split_whitespace() {
       let _ = std::process::Command::new("docker")
           .args(["rm", "-f", id])
           .output();
   }
   ```

4. Run the two targeted tests to confirm they pass:
   ```
   cargo test -p smelt-cli --test docker_lifecycle -- collect
   cargo test -p smelt-cli --test docker_lifecycle -- cli_run_lifecycle
   ```

## Must-Haves

- [ ] `test_collect_creates_target_branch` passes: mock script runs git commands successfully inside the container
- [ ] `test_cli_run_lifecycle` passes: the final container-absence assertion succeeds even with pre-existing orphaned containers on the daemon
- [ ] No other test is modified — only the two failing tests are touched
- [ ] All previously-passing tests continue to pass

## Verification

- `cargo test -p smelt-cli --test docker_lifecycle -- collect` → `test test_collect_creates_target_branch ... ok`
- `cargo test -p smelt-cli --test docker_lifecycle -- cli_run_lifecycle` → `test test_cli_run_lifecycle ... ok`
- `cargo test -p smelt-cli --test docker_lifecycle` → no `FAILED` lines

## Observability Impact

- Signals added/changed: None — these are test-only changes
- How a future agent inspects this: `cargo test -p smelt-cli --test docker_lifecycle` — a clean run means the baseline is healthy
- Failure state exposed: If the apk install step itself fails (network issue), the test will fail with a clear assertion message including the apk output

## Inputs

- `crates/smelt-cli/tests/docker_lifecycle.rs` — contains the two failing tests; lines ~220 (`test_cli_run_lifecycle`) and ~593 (`test_collect_creates_target_branch`)
- S06 Research: confirms 6 orphaned containers currently on daemon; confirms `apk add --no-cache git` works in ~2s in alpine:3

## Expected Output

- `crates/smelt-cli/tests/docker_lifecycle.rs` — two tests modified: git install added to `test_collect_creates_target_branch`, pre-clean added to `test_cli_run_lifecycle`
- Both tests now produce `ok` in the Docker integration test run
