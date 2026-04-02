---
estimated_steps: 5
estimated_files: 4
---

# T03: Wire mount + assay invocation into CLI run and verify with mock

**Slice:** S03 — Repo Mount & Assay Execution
**Milestone:** M001

## Description

Close the slice loop: replace the health-check in `execute_run()` with the full repo-mount + assay-invocation flow, and verify the end-to-end path with a mock assay script running inside an alpine container. The mock validates that the repo is mounted, the manifest file is present and readable, and session data survives the translation.

## Steps

1. Update `execute_run()` in `run.rs`: after provision (which now bind-mounts the repo), call `AssayInvoker::build_manifest_toml(&manifest)` → `AssayInvoker::write_manifest_to_container(&provider, &container, &toml)` → check write exit code → `AssayInvoker::build_run_command(&manifest)` → `provider.exec(&container, &cmd)` → check assay exit code → teardown. Keep the async block cleanup pattern (D026). Print lifecycle phases to stderr: "Mounting repo...", "Writing manifest...", "Executing assay run...", "Assay complete — exit code: N".
2. Add integration test `test_assay_mock_execution` in `docker_lifecycle.rs`: create a temp dir with a marker file (`echo "smelt-test" > repo/marker.txt`), build manifest pointing to temp dir with two sessions (one depends on the other), provision container → write manifest → write a mock script to the container via exec that: checks `/workspace/marker.txt` exists, reads `/tmp/smelt-manifest.toml`, parses session names from it, prints `MOCK_ASSAY: found N sessions: <names>`, exits 0 → exec the mock script instead of real assay → assert output contains expected session names and marker file confirmation.
3. Add integration test `test_assay_mock_failure` that writes a mock script exiting with code 1 and stderr output — verify `execute_run` surfaces the non-zero exit correctly.
4. Update `examples/job-manifest.toml`: change `job.repo` from URL to `"."` (current directory placeholder — dry-run doesn't resolve it). Add a comment noting that `smelt run` requires an absolute local path.
5. Update existing `test_cli_run_lifecycle` test to work with the new flow — it needs a real local repo path in the manifest. Use a temp dir as the repo path, write a manifest pointing to it, and verify the assay execution phase appears in stderr output (even though the real `assay` binary isn't present — the exec will fail, but the lifecycle messages should show the mount and manifest write phases before the assay command fails).

## Must-Haves

- [ ] `execute_run()` orchestrates: provision (with mount) → write manifest → exec assay → teardown
- [ ] Lifecycle phase messages printed to stderr for each step
- [ ] Mock assay test verifies mount is readable and manifest contains correct sessions
- [ ] Non-zero assay exit code is surfaced through the run result
- [ ] Existing tests updated and passing — no regressions
- [ ] Example manifest updated with local path guidance

## Verification

- `cargo test -p smelt-cli --test docker_lifecycle -- assay_mock` — mock assay integration tests pass
- `cargo test -p smelt-cli --test docker_lifecycle` — all Docker lifecycle tests pass
- `cargo test --workspace` — all tests pass, zero regressions
- `cargo run -- run examples/job-manifest.toml --dry-run` — dry-run still works with updated manifest

## Observability Impact

- Signals added/changed: stderr lifecycle messages for mount, manifest write, and assay execution phases; existing provisioning/teardown messages preserved
- How a future agent inspects this: `smelt run manifest.toml` stderr output shows each phase with timing/exit info; `SMELT_LOG=info` adds structured tracing for internal exec calls
- Failure state exposed: assay non-zero exit code reported with stderr content; manifest write failure reported with exec details

## Inputs

- `crates/smelt-core/src/assay.rs` — `AssayInvoker` with manifest translation and container writing (from T02)
- `crates/smelt-core/src/docker.rs` — `DockerProvider` with bind-mount provision and `/workspace` working_dir (from T01)
- `crates/smelt-cli/src/commands/run.rs` — existing `execute_run()` with async block cleanup pattern (from S02)
- `crates/smelt-cli/tests/docker_lifecycle.rs` — existing test helpers and patterns (from S02, T01)

## Expected Output

- `crates/smelt-cli/src/commands/run.rs` — `execute_run()` drives full mount + assay flow
- `crates/smelt-cli/tests/docker_lifecycle.rs` — new mock assay integration tests
- `examples/job-manifest.toml` — updated with local path repo field
