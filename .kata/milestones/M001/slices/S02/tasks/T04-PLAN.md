---
estimated_steps: 5
estimated_files: 4
---

# T04: Wire DockerProvider into CLI and run full lifecycle integration test

**Slice:** S02 — Docker Container Provisioning & Teardown
**Milestone:** M001

## Description

Closes the slice: converts the CLI main to async, wires `DockerProvider` into the `smelt run` command (non-dry-run path), and adds a CLI-level integration test proving the full lifecycle works from the real entrypoint. After this task, `smelt run manifest.toml` provisions a container, execs a health-check command, streams output, and tears down — the S02 demo is complete.

## Steps

1. Convert `crates/smelt-cli/src/main.rs`:
   - Add `#[tokio::main]` to `main()`, make it `async fn main()`.
   - Change `commands::run::execute(args)` to `commands::run::execute(args).await`.
2. Rewrite `crates/smelt-cli/src/commands/run.rs`:
   - Make `execute()` and `execute_dry_run()` async.
   - Add `execute_run()` async function for the non-dry-run path:
     - Load and validate manifest (same as dry-run).
     - Check `manifest.environment.runtime == "docker"` — error if not.
     - Instantiate `DockerProvider::new()?`.
     - Call `provider.provision(&manifest).await?` → get `container_id`.
     - Use a cleanup guard: ensure `provider.teardown(&container_id).await` runs regardless of what happens next. Use a manual match or scopeguard pattern — on any error after provision, teardown before returning the error.
     - Exec a health-check command: `["echo", "smelt: container ready"]` inside the container via `provider.exec()`.
     - Print success message with container ID and exit code.
     - Call `provider.teardown()`.
     - Return exit code 0 on success.
   - Note: Full session execution (running all manifest sessions via Assay) is S03. This task execs a single health-check command to prove the lifecycle works.
3. Update `examples/job-manifest.toml` if needed — ensure the image (`node:20-slim` or similar) is available or change to `alpine:3` for faster testing. The manifest should work for both dry-run and live run.
4. Add CLI-level integration test in `docker_lifecycle.rs`:
   - `test_cli_run_lifecycle`: use `assert_cmd` to run `smelt run examples/job-manifest.toml`, assert exit code 0, assert output contains "smelt: container ready" or equivalent health check text, confirm no containers remain after.
   - `test_cli_run_invalid_manifest`: run with bad manifest, assert exit code 1, assert error message present.
5. Final verification: run `cargo test --workspace` — all tests pass (existing 71 + new docker tests). Run `cargo run -- run examples/job-manifest.toml` manually to confirm the demo. Check `docker ps -a --filter label=smelt.job` returns empty.

## Must-Haves

- [ ] CLI main is async with `#[tokio::main]`
- [ ] `smelt run manifest.toml` (without `--dry-run`) drives DockerProvider lifecycle
- [ ] Teardown runs on both success and error paths (no leaked containers)
- [ ] Health-check command output is streamed to terminal
- [ ] Runtime type check: only "docker" is accepted, other values produce clear error
- [ ] CLI-level integration test passes via `assert_cmd`
- [ ] `cargo test --workspace` passes with all old and new tests

## Verification

- `cargo test --workspace` — all tests pass, zero warnings
- `cargo test -p smelt-cli -- docker_lifecycle::test_cli_run_lifecycle` passes
- `cargo run -- run examples/job-manifest.toml` — provisions, execs, streams output, tears down, exits 0
- `cargo run -- run examples/job-manifest.toml --dry-run` — still works (no regressions)
- `docker ps -a --filter label=smelt.job` — empty after all runs

## Observability Impact

- Signals added/changed: CLI prints lifecycle phase transitions (provisioning → executing → tearing down). Error messages include provider operation context.
- How a future agent inspects this: `smelt run manifest.toml` output shows each phase. Tracing at `info` level (via `SMELT_LOG=info`) shows full bollard operations.
- Failure state exposed: Provider instantiation failures (Docker not running), provision failures (image not found), exec failures (command error), and teardown failures all produce structured errors at the CLI level with exit code 1.

## Inputs

- `crates/smelt-core/src/docker.rs` — fully working `DockerProvider` with `provision()`, `exec()`, `teardown()` from T02/T03
- `crates/smelt-cli/src/main.rs` — current sync main
- `crates/smelt-cli/src/commands/run.rs` — current dry-run-only implementation
- `examples/job-manifest.toml` — valid manifest for testing

## Expected Output

- `crates/smelt-cli/src/main.rs` — async main with `#[tokio::main]`
- `crates/smelt-cli/src/commands/run.rs` — async `execute()` with DockerProvider lifecycle for non-dry-run
- `crates/smelt-cli/tests/docker_lifecycle.rs` — CLI-level integration tests added
- `examples/job-manifest.toml` — updated if needed for testability
