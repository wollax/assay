# S03: Repo Mount & Assay Execution — UAT

**Milestone:** M001
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: The slice's value is structural (bind-mount wiring, manifest translation, orchestration flow). Integration tests with mock scripts against real Docker validate the contract more reliably than manual inspection. No human-facing UI or experience to evaluate.

## Preconditions

- Docker daemon running and accessible
- Rust toolchain installed (`cargo` available)
- Repository checked out at the slice branch

## Smoke Test

Run `cargo test -p smelt-cli --test docker_lifecycle -- assay_mock_execution` — if it passes, the core mount + manifest + exec flow works end-to-end against real Docker.

## Test Cases

### 1. Bind-mount read fidelity

1. Create a temp directory with a marker file
2. Provision a container with the temp dir as repo path
3. Exec `cat /workspace/<marker>` inside the container
4. **Expected:** Output matches the marker file content written on the host

### 2. Bind-mount write fidelity

1. Provision a container with a temp dir mounted
2. Exec `echo "from-container" > /workspace/output.txt` inside container
3. Read the file from the host temp directory
4. **Expected:** Host file contains "from-container"

### 3. Mock assay execution with manifest

1. Create a temp repo with a marker file and a manifest with 2 sessions
2. Provision container, write assay manifest, run mock script
3. Mock script checks `/workspace` exists, reads `/tmp/smelt-manifest.toml`, prints session names
4. **Expected:** Mock output contains both session names and the marker file content; exit code 0

### 4. Assay failure propagation

1. Write a mock script that prints to stderr and exits with code 1
2. Execute via the assay invocation path
3. **Expected:** ExecHandle captures exit code 1 and stderr content

### 5. URL repo path rejection

1. Create a manifest with `job.repo = "https://github.com/example/repo.git"`
2. Attempt to provision
3. **Expected:** `SmeltError::Manifest` with field "job.repo" and message indicating URLs are not supported

## Edge Cases

### SCP-style SSH path rejection

1. Set `job.repo = "git@github.com:user/repo.git"`
2. Call `resolve_repo_path()`
3. **Expected:** Rejected as URL-like with clear error message

### Nonexistent local path

1. Set `job.repo = "/tmp/nonexistent-path-12345"`
2. Call `resolve_repo_path()`
3. **Expected:** `SmeltError::Manifest` with "cannot resolve repo path" and OS error

### Special characters in session spec

1. Create sessions with quotes, brackets, braces in the spec text
2. Build manifest TOML via `AssayInvoker`
3. **Expected:** TOML is valid and round-trips correctly through serialize/deserialize

## Failure Signals

- Any `cargo test --workspace` failure indicates regression
- `docker inspect` not showing a bind-mount in `Mounts` means provision wiring is broken
- Missing lifecycle messages on stderr (Writing manifest, Executing assay run, Assay complete) indicates `run.rs` orchestration flow is incomplete
- Non-zero assay exit without captured stderr means error propagation is broken

## Requirements Proved By This UAT

- No REQUIREMENTS.md exists — operating in legacy compatibility mode per M001-ROADMAP.md

## Not Proven By This UAT

- Real Assay CLI invocation (uses mock scripts, not actual `assay run`)
- Assay manifest format correctness against real Assay (deferred to S06)
- Result collection from container after Assay completes (S04)
- Timeout enforcement and graceful shutdown during execution (S05)
- Multi-session dependency ordering (S06)

## Notes for Tester

- All test cases are automated in `docker_lifecycle.rs` — no manual steps needed
- Docker daemon must be running; tests skip gracefully if unavailable (D024)
- The `test_cli_run_lifecycle` test does not assert success because alpine lacks an `assay` binary — it only verifies lifecycle messages appear before the expected failure
- The example manifest uses `"."` as repo path which works for `--dry-run` but actual execution needs an absolute path (canonicalized at runtime)
