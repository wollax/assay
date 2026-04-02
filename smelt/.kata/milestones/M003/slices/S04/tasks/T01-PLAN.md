---
estimated_steps: 7
estimated_files: 4
---

# T01: Migrate JobMonitor state path from flat file to per-job directories

**Slice:** S04 — Infrastructure Hardening
**Milestone:** M003

## Description

The flat `.smelt/run-state.toml` file (D034) was always a temporary single-job model. This task implements the migration to `.smelt/runs/<job-name>/state.toml`, enabling concurrent jobs to coexist without clobbering state. It also preserves backward compatibility for `smelt status` invocations against old flat state files.

The state file rename (`run-state.toml` → `state.toml`) affects `monitor.rs` write/read/cleanup. The state directory change (`.smelt` → `.smelt/runs/<job-name>`) affects `run.rs` (where state_dir is computed) and `watch.rs` (which currently ignores `args.job_name` when building the state path — the research bug to fix). `status.rs` gains an optional positional `job_name` argument; when absent, it reads the legacy flat path for backward compat.

This is the only task in S04 that modifies existing tested code, so it must leave all 124 pre-existing tests passing.

## Steps

1. **`monitor.rs` — rename file references.** In `write()`, change `self.state_dir.join("run-state.toml")` → `self.state_dir.join("state.toml")`. In `read()`, change `state_dir.join("run-state.toml")` → `state_dir.join("state.toml")`. In `cleanup()`, change `self.state_dir.join("run-state.toml")` → `self.state_dir.join("state.toml")`. Update the module doc comment to reflect the new path.

2. **`monitor.rs` — add `read_legacy()`.** Add `pub fn read_legacy(base_dir: &Path) -> Result<RunState>` that reads `{base_dir}/run-state.toml` using the same deserialization logic as `read()`. This provides backward compat for `smelt status` without a job name.

3. **`monitor.rs` — update unit tests.** In `test_run_state_toml_serialization`, change the file-existence check from `run-state.toml` to `state.toml`. Add three new unit tests:
   - `test_read_legacy_reads_flat_file`: writes a `RunState` to `{tmp}/run-state.toml` using `fs::write` (not `JobMonitor::write()`), then calls `JobMonitor::read_legacy(tmp)` and asserts fields match.
   - `test_state_path_resolution`: creates a `JobMonitor` with a state_dir of `.smelt/runs/my-job`, calls `write()`, asserts that `.smelt/runs/my-job/state.toml` exists (not `run-state.toml`).
   - `test_cleanup_uses_state_toml`: writes via `write()`, verifies `state.toml` exists, calls `cleanup()`, asserts `state.toml` is removed.

4. **`run.rs` — update state_dir computation.** After manifest is loaded (line where `state_dir` is currently set), change from:
   ```rust
   let state_dir = args.manifest.parent().unwrap_or_else(|| Path::new(".")).join(".smelt");
   ```
   to:
   ```rust
   let state_dir = args.manifest.parent().unwrap_or_else(|| Path::new("."))
       .join(".smelt").join("runs").join(&manifest.job.name);
   ```
   `JobMonitor::write()` already calls `fs::create_dir_all(&self.state_dir)` on every write, so the nested directory creation is automatic — no extra `mkdir` needed.

5. **`watch.rs` — fix `execute()` state path.** Change `let state_dir = PathBuf::from(".").join(".smelt");` to `let state_dir = PathBuf::from(".").join(".smelt").join("runs").join(&args.job_name);`. Update `persist_run_state()` to write to `state_dir.join("state.toml")` instead of `state_dir.join("run-state.toml")`. Update the `run_watch()` state update block: `persist_run_state()` already receives `state_dir` — no change needed there, only the filename fix in `persist_run_state()` itself. Update watch test helper `write_state_to_dir()` to write `state.toml` (not `run-state.toml`) in the created `.smelt` directory — tests use a mock state_dir that already mirrors the per-job layout.

6. **`status.rs` — add optional positional `job_name` arg.** Add `/// Job name to read (reads per-job state). Omit to read legacy flat state for backward compat. #[arg] pub job_name: Option<String>` to `StatusArgs`. In `execute()`, derive `state_dir_base = args.dir.join(".smelt")`; when `job_name` is `Some(name)`, set `state_dir = state_dir_base.join("runs").join(name)` and call `JobMonitor::read(&state_dir)`; when `None`, call `JobMonitor::read_legacy(&state_dir_base)`. Update `smelt status` help text: `Pass job-name to read per-job state; omit to read legacy flat state for backward compat.` Update the inline test helper `write_state()` to accept an explicit `state_dir: &Path` and write `state_dir.join("state.toml")` (the test for active job with live PID must set up a per-job path, while the backward-compat test can use the legacy flat path).

7. **Compile + test.** Run `cargo test -p smelt-core && cargo test -p smelt-cli`. Fix any compilation errors. Confirm all 124+ tests pass (including the newly added ones).

## Must-Haves

- [ ] `JobMonitor::write()` writes to `{state_dir}/state.toml` (not `run-state.toml`)
- [ ] `JobMonitor::read()` reads from `{state_dir}/state.toml`
- [ ] `JobMonitor::cleanup()` removes `{state_dir}/state.toml`
- [ ] `JobMonitor::read_legacy(base_dir)` reads `{base_dir}/run-state.toml`
- [ ] `run.rs` state_dir is `.smelt/runs/<manifest.job.name>` (not `.smelt`)
- [ ] `watch.rs` `execute()` uses `.smelt/runs/<args.job_name>` as state_dir
- [ ] `watch.rs` `persist_run_state()` writes to `state.toml` (not `run-state.toml`)
- [ ] `status.rs` optional positional `job_name: Option<String>` in `StatusArgs`
- [ ] `status.rs` `execute()` calls `read_legacy()` when job_name is None; calls `read()` with per-job path when job_name is Some
- [ ] `cargo test -p smelt-core && cargo test -p smelt-cli` — 0 failures

## Verification

- `cargo test -p smelt-core` — all monitor tests pass, including the three new ones
- `cargo test -p smelt-cli` — all tests pass, including existing status and watch tests
- In `test_state_path_resolution`: `TempDir::new()`, create `JobMonitor::new("my-job", vec![], tmp.join(".smelt/runs/my-job"))`, call `write()`, assert `tmp.join(".smelt/runs/my-job/state.toml").exists()` is true
- In `test_read_legacy_reads_flat_file`: manually write a TOML string to `tmp.join("run-state.toml")`, call `JobMonitor::read_legacy(tmp)`, assert fields match

## Observability Impact

- Signals added/changed: `smelt status` now accepts `job_name` positional arg — the command's help text surfaces the backward-compat behavior
- How a future agent inspects this: `smelt status <job-name>` reads `.smelt/runs/<name>/state.toml`; `smelt status` (no args) reads legacy `.smelt/run-state.toml`; `smelt list` (T03) will read all per-job state files
- Failure state exposed: if `smelt watch <job-name>` is called before `smelt run` creates the per-job directory, the existing "No state file" error propagates with the new path in context

## Inputs

- `crates/smelt-core/src/monitor.rs` — `write()`, `read()`, `cleanup()` all hardcode `run-state.toml`; `state_dir` is the only stored path
- `crates/smelt-cli/src/commands/run.rs` — state_dir line is the only change; `JobMonitor::new()` call immediately follows
- `crates/smelt-cli/src/commands/watch.rs` — `execute()` hardcodes `.smelt` as state_dir; `persist_run_state()` hardcodes `run-state.toml`
- `crates/smelt-cli/src/commands/status.rs` — only takes `--dir`; calls `JobMonitor::read()` not `read_legacy()`
- `examples/.smelt/run-state.toml` — legacy flat state example; keep as-is (documents the legacy path)

## Expected Output

- `crates/smelt-core/src/monitor.rs` — `write/read/cleanup` use `state.toml`; new `read_legacy()` method; 3 new unit tests
- `crates/smelt-cli/src/commands/run.rs` — state_dir uses `.smelt/runs/<job.name>`
- `crates/smelt-cli/src/commands/watch.rs` — `execute()` uses per-job path; `persist_run_state()` writes `state.toml`
- `crates/smelt-cli/src/commands/status.rs` — optional positional `job_name`; backward-compat `read_legacy()` call when absent
