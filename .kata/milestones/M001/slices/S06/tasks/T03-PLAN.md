---
estimated_steps: 5
estimated_files: 1
---

# T03: Add multi-session and error-path integration tests

**Slice:** S06 — End-to-End Integration
**Milestone:** M001

## Description

Two remaining S06 must-haves are not yet covered after T02:

1. **Multi-session manifest verification** — No test exercises a manifest with 2+ sessions and `depends_on`. `test_multi_session_e2e` provisions a container, writes a 2-session manifest, verifies both session names appear in the serialized TOML written to the container, and confirms mock assay executes successfully.

2. **Error path** — `test_e2e_assay_failure_no_orphans` verifies that when assay exits non-zero, the container is torn down and no orphaned containers remain. This closes the error branch in `execute_run()`: assay non-zero exit → bail → teardown → cleanup.

Neither test requires git or result collection — they focus on the manifest serialization path and the teardown guarantee on failure.

## Steps

1. Open `crates/smelt-cli/tests/docker_lifecycle.rs`. Add both tests after `test_full_e2e_pipeline`.

2. **`test_multi_session_e2e`**:
   - Guard with `docker_provider_or_skip()`.
   - Build manifest with two sessions:
     ```rust
     let mut manifest = test_manifest("multi-session-e2e");
     manifest.session = vec![
         SessionDef {
             name: "session-one".to_string(),
             spec: "spec-one".to_string(),
             harness: "echo one".to_string(),
             timeout: 60,
             depends_on: vec![],
         },
         SessionDef {
             name: "session-two".to_string(),
             spec: "spec-two".to_string(),
             harness: "echo two".to_string(),
             timeout: 60,
             depends_on: vec!["session-one".to_string()],
         },
     ];
     ```
   - Provision container.
   - Write mock assay to `/usr/local/bin/assay` (script that verifies `/tmp/smelt-manifest.toml` exists, then exits 0 — no git needed):
     ```sh
     #!/bin/sh
     set -e
     test -f /tmp/smelt-manifest.toml || { echo "manifest missing"; exit 1; }
     exit 0
     ```
   - Write manifest via `AssayInvoker::write_manifest_to_container`.
   - Read manifest back from container: `provider.exec(&container, &["cat", "/tmp/smelt-manifest.toml"])` — assert exit_code == 0.
   - Assert stdout contains both `"session-one"` and `"session-two"`.
   - Assert stdout contains `"session-one"` in a `depends_on` context (i.e. `depends_on = ["session-one"]` appears in the TOML).
   - Execute assay: `provider.exec(&container, &AssayInvoker::build_run_command(&manifest))` — assert exit_code == 0.
   - Teardown and assert container removed.

3. **`test_e2e_assay_failure_no_orphans`**:
   - Guard with `docker_provider_or_skip()`.
   - Build manifest: `test_manifest("failure-no-orphans")`.
   - Provision container.
   - Write failing mock assay to `/usr/local/bin/assay`:
     ```sh
     #!/bin/sh
     exit 1
     ```
   - Write smelt manifest via `AssayInvoker::write_manifest_to_container`.
   - Exec assay: `provider.exec(&container, &AssayInvoker::build_run_command(&manifest))` — assert exit_code == 1.
   - Call `provider.teardown(&container)` — assert `Ok(())`.
   - Assert container removed: `assert_container_removed(&provider, container.as_str()).await`.
   - Confirm no orphaned smelt containers remain: run `docker ps -aq --filter label=smelt.job`, assert output is empty (trim whitespace).

4. Run the new tests:
   ```
   cargo test -p smelt-cli --test docker_lifecycle -- multi_session
   cargo test -p smelt-cli --test docker_lifecycle -- failure_no_orphans
   ```

5. Run the full suite and confirm zero failures:
   ```
   cargo test --workspace 2>&1 | tail -10
   ```

## Must-Haves

- [ ] `test_multi_session_e2e`: manifest written to container contains both session names and the `depends_on` relationship
- [ ] `test_multi_session_e2e`: mock assay executed via `build_run_command` exits 0
- [ ] `test_e2e_assay_failure_no_orphans`: assay exits 1, container is still torn down, `docker ps --filter label=smelt.job` returns empty
- [ ] `cargo test --workspace` passes with zero failures after both tests are added

## Verification

- `cargo test -p smelt-cli --test docker_lifecycle -- multi_session` → `ok`
- `cargo test -p smelt-cli --test docker_lifecycle -- failure_no_orphans` → `ok`
- `cargo test --workspace 2>&1 | grep -E "^test result"` → all `ok`, 0 failed

## Observability Impact

- Signals added/changed: `test_e2e_assay_failure_no_orphans` explicitly asserts the post-failure `docker ps` output — this is a direct observability check that teardown-on-failure is working; failure produces a clear assertion message with the list of leaked container IDs
- How a future agent inspects this: `cargo test -p smelt-cli --test docker_lifecycle -- --nocapture` shows all docker ps output; if the failure-path test fails, the leaked container ID is in the assertion message
- Failure state exposed: `test_multi_session_e2e` prints the full manifest TOML on assertion failure so missing fields are immediately visible

## Inputs

- `crates/smelt-cli/tests/docker_lifecycle.rs` — existing helpers (`docker_provider_or_skip`, `test_manifest`, `assert_container_removed`, `AssayInvoker` import) established in T01/T02
- `crates/smelt-core/src/manifest.rs` — `SessionDef` struct fields: `name`, `spec`, `harness`, `timeout`, `depends_on`
- `crates/smelt-core/src/assay.rs` — `AssayInvoker::build_manifest_toml()` serializes sessions + depends_on to TOML; `build_run_command()` produces `["assay", "run", "/tmp/smelt-manifest.toml", "--timeout", "<max>"]`
- S06 Research: "mock assay must exit 0 for success path"; "multi-session test only needs to verify manifest round-trip, not that Assay respects dependency ordering (that's Assay's concern per D002)"
- T02 output — `test_full_e2e_pipeline` established the mock assay pattern; T03 reuses same base64 write approach

## Expected Output

- `crates/smelt-cli/tests/docker_lifecycle.rs` — two new tests (`test_multi_session_e2e` ~60 lines, `test_e2e_assay_failure_no_orphans` ~45 lines)
- `cargo test --workspace` — all tests pass; M001 milestone definition of done is satisfied
