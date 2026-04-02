# S03: Streaming Assay Output

**Goal:** `smelt run` prints assay gate output lines to stderr as they are produced — not buffered until assay exits. The double-print bug (every line appearing twice) is eliminated. `exec_streaming()` is a first-class method on `RuntimeProvider` and `DockerProvider`.
**Demo:** `cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order -- --nocapture` passes — callback is called per chunk in order, and `ExecHandle` still carries the full buffered output. `cargo test --workspace` passes clean.

## Must-Haves

- `RuntimeProvider` trait gains `exec_streaming<F>(..., output_cb: F)` method with `F: FnMut(&str) + Send + 'static`
- `DockerProvider::exec_streaming()` calls the callback for each stdout/stderr chunk as bollard delivers it; also populates `ExecHandle.stdout`/`ExecHandle.stderr` for error reporting
- `exec()` in `DockerProvider` no longer calls `eprint!` — setup command output goes silent (correct: output is available on `ExecHandle` for diagnostics)
- Phase 7 in `execute_run()` uses `exec_streaming(&container, &cmd, |chunk| eprint!("{chunk}"))` instead of `exec()`
- The post-exec `if !handle.stdout.is_empty() { eprint!(...) }` / `if !handle.stderr.is_empty() { eprint!(...) }` block is removed from Phase 7
- `test_exec_streaming_delivers_chunks_in_order` in `docker_lifecycle.rs` passes: provisions container, runs `printf 'a\nb\nc\n'`, collects chunks via callback, asserts order, and asserts `ExecHandle.stdout` is populated
- All existing tests pass unchanged (`cargo test --workspace` clean)

## Proof Level

- This slice proves: operational (real bollard streaming path, real Docker container, real incremental chunk delivery)
- Real runtime required: yes (Docker daemon; test skips gracefully if unavailable)
- Human/UAT required: no (automated test proves streaming contract; observable via `--nocapture`)

## Verification

```bash
# Full workspace — all test suites pass
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["
# → each suite shows "test result: ok." — no FAILED, no error[]

# Streaming unit test — proves callback receives chunks in order
cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order -- --nocapture
# → chunks ["a\n", "b\n", "c\n"] collected in order (or as a single chunk; order preserved)
# → handle.stdout contains "a\nb\nc\n"
# → test result: ok. 1 passed

# Smoke: existing exec tests unaffected
cargo test -p smelt-core 2>&1 | grep -E "^test result|FAILED"
# → test result: ok.

# No double-print: exec() no longer has eprint! — grep confirms removal
grep -n "eprint!" crates/smelt-core/src/docker.rs
# → no lines matching (eprint removed from exec loop)
```

## Observability / Diagnostics

- Runtime signals: assay run output streams to stderr in real time via `eprint!("{chunk}")` in Phase 7 callback; each chunk arrives as bollard delivers it (no buffering until assay exits)
- Inspection surfaces: `--nocapture` test output shows "chunk[0] = ...", "chunk[1] = ...", "ExecHandle.stdout = ..." from `test_exec_streaming_delivers_chunks_in_order`
- Failure visibility: if callback is never called, test asserts `!chunks.is_empty()` and fails with clear message; if `ExecHandle.stdout` is empty, separate assertion fails; if Phase 7 exits non-zero, `handle.stderr.trim()` is included in the bail message (unchanged from S02)
- Redaction constraints: none — no secrets flow through the streaming callback

## Integration Closure

- Upstream surfaces consumed: `DockerProvider::exec()` bollard loop (extracted into `exec_streaming()`), `RuntimeProvider` trait, Phase 7 of `execute_run()` in `run.rs`
- New wiring introduced in this slice: `exec_streaming()` on trait + `DockerProvider`; Phase 7 wired to callback-based streaming; `eprint!` moved from inside `exec()` to Phase 7 caller
- What remains before the milestone is truly usable end-to-end: S04 (exit code 2 distinction + `ResultCollector` verification)

## Tasks

- [x] **T01: Add `exec_streaming()` to `RuntimeProvider` trait and `DockerProvider`** `est:45m`
  - Why: The transport layer (bollard chunk stream) already streams; this task exposes it via a clean callback API and silences the inline `eprint!` from `exec()` so setup commands stop polluting the terminal
  - Files: `crates/smelt-core/src/provider.rs`, `crates/smelt-core/src/docker.rs`
  - Do:
    1. In `provider.rs`, add `exec_streaming` method to `RuntimeProvider` trait with signature `fn exec_streaming<F>(&self, container: &ContainerId, command: &[String], output_cb: F) -> impl Future<Output = crate::Result<ExecHandle>> + Send where F: FnMut(&str) + Send + 'static`. Use the RPITIT pattern (D019) matching existing trait methods.
    2. In `docker.rs`, implement `exec_streaming` on `DockerProvider`. Copy the `create_exec` → `start_exec` → `inspect_exec` skeleton from `exec()`. In the `while let Some(chunk) = output.next().await` loop, call `output_cb(&text)` instead of `eprint!("{text}")` for both `StdOut` and `StdErr` chunks; still push to `stdout_buf`/`stderr_buf` for the returned `ExecHandle`.
    3. In `docker.rs` `exec()`, remove the two `eprint!("{text}");` lines inside the `StdOut` and `StdErr` match arms. Retain `debug!` logging and all buffer pushes. `exec()` becomes silent — output is available on the returned `ExecHandle`.
    4. Confirm the `'static` bound on `F` is sufficient: the callback is moved into the async block, so no lifetime shorter than `'static` is needed for RPITIT futures.
  - Verify: `cargo test -p smelt-core` passes. `grep -n "eprint!" crates/smelt-core/src/docker.rs` returns no matches.
  - Done when: `cargo test -p smelt-core` is green; `exec()` has no `eprint!` calls; `exec_streaming()` exists on both trait and impl with correct signature

- [x] **T02: Wire Phase 7 to `exec_streaming()` and add streaming integration test** `est:45m`
  - Why: Phase 7 currently uses buffered `exec()` and double-prints all assay output; switching to `exec_streaming()` with `|chunk| eprint!("{chunk}")` makes output real-time and eliminates the redundant post-exec eprint block; the integration test proves the callback delivers chunks in order against a real container
  - Files: `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do:
    1. In `run.rs` Phase 7, inside `exec_future`, replace `provider.exec(&container, &cmd).await` with `provider.exec_streaming(&container, &cmd, |chunk| eprint!("{chunk}")).await`. The return value is still an `ExecHandle`.
    2. Delete the post-exec output block that follows (the `if !handle.stdout.is_empty() { eprint!("{}", handle.stdout); }` and `if !handle.stderr.is_empty() { eprint!("{}", handle.stderr); }` lines). Output already reached the terminal via the callback.
    3. Retain the `handle.exit_code != 0` bail path — it uses `handle.stderr.trim()` in the error message, which is still populated (streaming variant buffers for diagnostics per T01).
    4. In `docker_lifecycle.rs`, add test `test_exec_streaming_delivers_chunks_in_order`. Pattern: skip if Docker unavailable (existing `check_docker` pattern), provision an `alpine:3` container with a minimal manifest, run `printf 'a\nb\nc\n'` via `provider.exec_streaming()` collecting chunks into an `Arc<Mutex<Vec<String>>>`, tear down, assert `!chunks.is_empty()`, assert joined output equals `"a\nb\nc\n"`, assert `handle.stdout` contains `"a"`.
    5. Use `Arc<Mutex<Vec<String>>>` for the chunk accumulator in the test so the `FnMut + Send + 'static` bounds are satisfied without lifetime conflicts.
  - Verify: `cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order -- --nocapture` passes and prints collected chunks. `cargo test --workspace` clean.
  - Done when: streaming test passes; Phase 7 uses `exec_streaming`; post-exec eprint block is gone; `cargo test --workspace` green

## Files Likely Touched

- `crates/smelt-core/src/provider.rs`
- `crates/smelt-core/src/docker.rs`
- `crates/smelt-cli/src/commands/run.rs`
- `crates/smelt-cli/tests/docker_lifecycle.rs`
