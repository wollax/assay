# S04: Infrastructure Hardening — UAT

**Milestone:** M003
**Written:** 2026-03-21

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All S04 deliverables are testable without Docker, real GitHub tokens, or live containers. Per-job state path resolution, `smelt init`, `smelt list`, and the gitignore guard are all exercised by `cargo test` using `TempDir`-based fixtures. The slice plan explicitly states "Real runtime required: no" and "Human/UAT required: no."

## Preconditions

- Rust toolchain installed (`cargo` on PATH)
- `smelt` binary built: `cargo build -p smelt-cli`
- Working directory: any directory on the host filesystem
- Docker NOT required

## Smoke Test

```
cargo build -p smelt-cli && \
mkdir /tmp/smelt-uat-s04 && cd /tmp/smelt-uat-s04 && \
/path/to/target/debug/smelt init && \
/path/to/target/debug/smelt run job-manifest.toml --dry-run
```

Expected: `smelt init` exits 0, creates `job-manifest.toml`; `smelt run --dry-run` exits 0 and prints the execution plan.

## Test Cases

### 1. smelt init creates a valid manifest

1. Change to an empty temporary directory.
2. Run `smelt init`.
3. Verify `job-manifest.toml` was created.
4. Run `smelt run job-manifest.toml --dry-run`.
5. **Expected:** `smelt init` exits 0 and prints "Next: smelt run job-manifest.toml"; `smelt run --dry-run` exits 0 and prints the execution plan with no validation errors.

### 2. smelt init idempotency guard

1. In the same temporary directory (file already exists).
2. Run `smelt init` again.
3. **Expected:** exits 1 with a clear error message mentioning "already exists" on stderr; the existing file is unchanged.

### 3. smelt run writes to per-job state directory

1. Run `smelt run job-manifest.toml --dry-run` (or a real run if Docker available) in a directory.
2. After completion, check for `.smelt/runs/<job-name>/state.toml` (where `<job-name>` is the `job.name` field in the manifest).
3. **Expected:** `.smelt/runs/<job-name>/state.toml` exists; `.smelt/run-state.toml` does NOT exist at the top level.

### 4. smelt status backward compatibility

1. Create `.smelt/run-state.toml` manually with valid RunState TOML content.
2. Run `smelt status` (no arguments).
3. **Expected:** reads and displays the legacy flat file without error.
4. Also run `smelt status <job-name>` with a per-job state directory present.
5. **Expected:** reads from `.smelt/runs/<job-name>/state.toml`.

### 5. smelt list with no past runs

1. In a directory without a `.smelt/runs/` subdirectory.
2. Run `smelt list`.
3. **Expected:** prints "No past runs." and exits 0.

### 6. smelt list shows past runs

1. In a directory with `.smelt/runs/<job-name>/state.toml` present (from a prior `smelt run`).
2. Run `smelt list`.
3. **Expected:** prints a header row and at least one data row showing job name, phase, elapsed time, and PR URL (or `-` if absent). Exits 0.

### 7. smelt list skips corrupt state file

1. Create `.smelt/runs/broken-job/state.toml` with invalid TOML content.
2. Run `smelt list`.
3. **Expected:** prints a `[WARN] skipping ...` message to stderr; does not abort; exits 0.

## Edge Cases

### .assay/ gitignore guard — creates new file

1. Change to an empty directory with no `.gitignore`.
2. Trigger `smelt run` (or call `ensure_gitignore_assay()` via unit test path).
3. **Expected:** `.gitignore` is created containing `.assay/`.

### .assay/ gitignore guard — appends to existing file

1. Change to a directory with `.gitignore` that does NOT contain `.assay/`.
2. Trigger `smelt run`.
3. **Expected:** `.assay/` is appended to the existing `.gitignore`; existing entries are unchanged.

### .assay/ gitignore guard — idempotent

1. Run `smelt run` twice in the same directory.
2. **Expected:** `.assay/` appears exactly once in `.gitignore`; no duplication.

### smelt list with --dir flag

1. Run `smelt list --dir /path/to/repo`.
2. **Expected:** looks for `.smelt/runs/` under the specified path, not the current working directory.

## Failure Signals

- `smelt init` exits 0 but produces a manifest that fails `--dry-run` validation → skeleton SKELETON const is malformed (check `JobManifest::validate()` constraints)
- `smelt run` writes to `.smelt/run-state.toml` (flat path) instead of `.smelt/runs/<name>/state.toml` → T01 regression in `run.rs` state_dir computation
- `smelt status` fails for a legacy `.smelt/run-state.toml` when called without args → `read_legacy()` path broken in `status.rs`
- `smelt list` aborts on a corrupt state.toml instead of printing a warning → error handling regression in `list.rs`
- `.assay/` appears twice in `.gitignore` after two runs → idempotency check in `ensure_gitignore_assay()` broken

## Requirements Proved By This UAT

- R006 — Concurrent smelt runs use isolated state directories: per-job path isolation verified by Test Case 3; backward compat verified by Test Case 4.
- R007 — smelt init generates a skeleton manifest: Test Cases 1 and 2 prove creation and idempotency guard.
- R008 — .assay/ protected from accidental git commits: Edge Cases 1–3 prove guard creates, appends, and is idempotent.

## Not Proven By This UAT

- Live Docker run writing state to per-job directory under real execution — requires Docker daemon; deferred to S06 integration proof.
- `smelt watch <job-name>` reading from per-job state after a real `smelt run` — requires real GitHub token + PR; deferred to S06.
- `smelt list --dir` on a repo with multiple concurrent job states — functional correctness covered by unit tests; real multi-job smoke test deferred to S06.
- `test_cli_run_invalid_manifest` failure in docker_lifecycle tests — pre-existing unrelated failure; not a S04 regression.

## Notes for Tester

- The `job.name` field in the generated `job-manifest.toml` skeleton is `"my-project"` — the per-job state path after `smelt run` will be `.smelt/runs/my-project/state.toml`.
- `smelt run --dry-run` validates the manifest without touching Docker or the network. Use this to confirm `smelt init` output is valid without any infrastructure.
- All unit tests for this slice can be run with `cargo test -p smelt-core && cargo test -p smelt-cli --lib`. The docker_lifecycle integration tests require Docker and will be slower.
