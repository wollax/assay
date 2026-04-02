---
estimated_steps: 5
estimated_files: 2
---

# T02: Wire Phase 7 to `exec_streaming()` and add streaming integration test

**Slice:** S03 — Streaming Assay Output
**Milestone:** M002

## Description

Phase 7 in `execute_run()` currently calls `provider.exec(&container, &cmd)` (buffered) and then re-prints the buffered output via a post-exec `eprint!` block — causing every line of assay output to appear twice. This task replaces that call with `provider.exec_streaming(&container, &cmd, |chunk| eprint!("{chunk}"))` and deletes the redundant post-exec eprint block. Additionally, a Docker integration test is added to `docker_lifecycle.rs` that proves `exec_streaming()` delivers chunks in order and that `ExecHandle` is still populated.

## Steps

1. **Replace Phase 7 `exec()` call with `exec_streaming()`** (`run.rs`): Inside the `exec_future` async block in Phase 7, change:
   ```rust
   let handle = provider
       .exec(&container, &cmd)
       .await
       .with_context(|| "failed to execute assay run")?;
   ```
   to:
   ```rust
   let handle = provider
       .exec_streaming(&container, &cmd, |chunk| eprint!("{chunk}"))
       .await
       .with_context(|| "failed to execute assay run")?;
   ```

2. **Delete the post-exec eprint block** (`run.rs`): Remove the following lines that immediately follow `eprintln!("Assay complete — exit code: {}", handle.exit_code);`:
   ```rust
   if !handle.stdout.is_empty() {
       eprint!("{}", handle.stdout);
   }
   if !handle.stderr.is_empty() {
       eprint!("{}", handle.stderr);
   }
   ```
   The `handle.exit_code != 0` bail path that comes after must remain — it uses `handle.stderr.trim()` in the error message, which is still valid because `exec_streaming()` populates `ExecHandle.stderr`.

3. **Add `test_exec_streaming_delivers_chunks_in_order`** (`docker_lifecycle.rs`): Follow the existing Docker skip + provision + teardown pattern. Use `printf 'a\nb\nc\n'` as the command (already available in alpine:3). Collect chunks using an `Arc<Mutex<Vec<String>>>` cloned into the callback closure to satisfy `FnMut + Send + 'static`. After teardown (before assertions — D039 teardown-before-assert pattern), assert:
   - `!chunks.is_empty()`
   - joined chunks equal `"a\nb\nc\n"` (order preserved; number of chunks may vary by bollard delivery)
   - `handle.stdout` contains `"a"` (ExecHandle is populated)
   Print chunks and `handle.stdout` before assertions for diagnosability without `--nocapture`.

4. **Ensure test manifest is minimal** (`docker_lifecycle.rs`): The streaming test only needs `provider.exec_streaming()`, not a full assay setup. Reuse the `make_test_manifest()` helper (or inline a minimal `JobManifest` with `alpine:3` and a local repo path) — the same pattern used by `test_run_without_dry_run_attempts_docker`.

5. **Verify full workspace** (`run.rs`, `docker_lifecycle.rs`): `cargo test --workspace` passes. `cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order -- --nocapture` passes and prints chunk details.

## Must-Haves

- [ ] Phase 7 uses `exec_streaming()` with `|chunk| eprint!("{chunk}")` callback
- [ ] Post-exec `if !handle.stdout.is_empty()` / `if !handle.stderr.is_empty()` eprint block is deleted from Phase 7
- [ ] `handle.exit_code != 0` bail path remains intact with `handle.stderr.trim()` in error message
- [ ] `test_exec_streaming_delivers_chunks_in_order` is present in `docker_lifecycle.rs`
- [ ] Test uses `Arc<Mutex<Vec<String>>>` for chunk accumulation (satisfies `Send + 'static` bounds)
- [ ] Test asserts chunk order and `ExecHandle.stdout` population
- [ ] Test tears down container before asserting (teardown-before-assert pattern)
- [ ] `cargo test --workspace` clean

## Verification

```bash
# Full workspace passes
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["
# → all "test result: ok." — no FAILED

# Streaming test with output
cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order -- --nocapture
# → prints chunks and handle.stdout
# → test result: ok. 1 passed

# Confirm post-exec eprint block is gone from Phase 7
grep -A5 "Assay complete" crates/smelt-cli/src/commands/run.rs
# → no "handle.stdout" or "handle.stderr" eprint lines in that block

# Confirm Phase 7 uses exec_streaming
grep "exec_streaming" crates/smelt-cli/src/commands/run.rs
# → at least one match in Phase 7
```

## Observability Impact

- Signals added/changed: assay run output now streams to user's stderr in real time (each chunk as bollard delivers it) rather than appearing all at once after assay exits. The "Assay complete — exit code: N" line still appears after streaming completes.
- How a future agent inspects this: `test_exec_streaming_delivers_chunks_in_order` output (with `--nocapture`) shows exact chunks received; failure message from `!chunks.is_empty()` assertion identifies if callback was never invoked.
- Failure state exposed: if `exec_streaming()` fails in Phase 7, the error propagates to `exec_future` which returns `ExecOutcome::Completed(Err(e))` → `monitor.set_phase(JobPhase::Failed)` → teardown → return Err. The `handle.stderr.trim()` in the bail message is still populated by the streaming variant's internal buffer.

## Inputs

- `crates/smelt-core/src/provider.rs`, `crates/smelt-core/src/docker.rs` — T01 output: `exec_streaming<F>()` on trait + impl with `F: FnMut(&str) + Send + 'static`
- `crates/smelt-cli/src/commands/run.rs` — Phase 7 block with buffered `exec()` call and post-exec eprint block (lines ~227–250)
- `crates/smelt-cli/tests/docker_lifecycle.rs` — existing Docker skip pattern, `make_test_manifest()` helper or inline minimal manifest

## Expected Output

- `crates/smelt-cli/src/commands/run.rs` — Phase 7 uses `exec_streaming()`; post-exec eprint block removed
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `test_exec_streaming_delivers_chunks_in_order` added with `Arc<Mutex<Vec<String>>>` accumulator and three assertions
