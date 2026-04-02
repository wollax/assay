---
id: T02
parent: S03
milestone: M002
provides:
  - Phase 7 uses exec_streaming() with live chunk callback — double-print bug eliminated
  - test_exec_streaming_delivers_chunks_in_order in docker_lifecycle.rs
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - none new — follows D019 RPITIT pattern from T01
patterns_established:
  - Arc<Mutex<Vec<String>>> for chunk accumulation in async test callbacks (satisfies Send + 'static)
  - teardown-before-assert ordering confirmed in streaming test (D039)
observability_surfaces:
  - test_exec_streaming_delivers_chunks_in_order prints chunk[i] = ... and handle.stdout = ... before assertions
  - assay run output now streams to stderr in real time via eprint!("{chunk}") in Phase 7 callback
duration: short
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Wire Phase 7 to `exec_streaming()` and add streaming integration test

**Replaced Phase 7's buffered `exec()` call with `exec_streaming()`, eliminated the double-print post-exec eprint block, and added `test_exec_streaming_delivers_chunks_in_order` to `docker_lifecycle.rs`.**

## What Happened

Phase 7 in `execute_run()` previously called `provider.exec(&container, &cmd)` (buffered) and then re-printed the captured stdout/stderr after the exec completed — causing every line of assay output to appear twice. This was replaced with:

```rust
let handle = provider
    .exec_streaming(&container, &cmd, |chunk| eprint!("{chunk}"))
    .await
    .with_context(|| "failed to execute assay run")?;
```

The two post-exec eprint blocks (`if !handle.stdout.is_empty()` / `if !handle.stderr.is_empty()`) were deleted. The `handle.exit_code != 0` bail path was left intact — `exec_streaming()` still populates `ExecHandle.stderr` for the error message.

The streaming integration test uses `printf 'a\nb\nc\n'` (available in alpine:3) and collects chunks into an `Arc<Mutex<Vec<String>>>` cloned into the callback closure to satisfy `FnMut + Send + 'static`. Teardown runs before assertions. The test confirms chunks are non-empty, joined chunks equal `"a\nb\nc\n"`, and `handle.stdout` is populated.

## Verification

```
# Full workspace
cargo test --workspace → all "test result: ok." — no FAILED

# Streaming test with output
cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order -- --nocapture
chunk[0] = "a\nb\nc\n"
handle.stdout = "a\nb\nc\n"
test test_exec_streaming_delivers_chunks_in_order ... ok
test result: ok. 1 passed

# Post-exec eprint block gone
grep -A5 "Assay complete" run.rs → no handle.stdout/stderr eprint lines

# Phase 7 uses exec_streaming
grep "exec_streaming" run.rs → one match in Phase 7
```

Slice verification checks status:
- ✅ `cargo test --workspace` — all ok
- ✅ `test_exec_streaming_delivers_chunks_in_order` passes with chunk output
- ✅ `cargo test -p smelt-core` — test result: ok.
- ✅ `grep "eprint!" crates/smelt-core/src/docker.rs` — no lines (removed in T01)

## Diagnostics

- `test_exec_streaming_delivers_chunks_in_order` prints `chunk[i] = ...` and `handle.stdout = ...` before assertions — visible with `--nocapture`
- Failure of `!chunks.is_empty()` assertion indicates callback was never invoked
- Failure of `handle.stdout.contains("a")` assertion indicates ExecHandle not populated
- Phase 7 assay output now appears on stderr in real time as bollard delivers chunks

## Deviations

none

## Known Issues

none

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — Phase 7 uses `exec_streaming()`; post-exec eprint block removed
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `test_exec_streaming_delivers_chunks_in_order` added; `Arc<Mutex>` import added
