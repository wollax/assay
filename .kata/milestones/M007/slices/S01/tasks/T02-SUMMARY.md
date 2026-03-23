---
id: T02
parent: S01
milestone: M007
provides:
  - launch_agent_streaming as a real pub function in assay-core::pipeline (replaces todo! stub)
key_files:
  - crates/assay-core/src/pipeline.rs
key_decisions:
  - stderr inherits from parent process (visible in terminal) — no change needed for streaming use case
  - Empty cli_args panics with assert! (clear message) — callers always provide at least the binary name
patterns_established:
  - Streaming primitive pattern — spawn child with piped stdout, BufReader::lines() in background thread, send via mpsc::Sender<String>, return JoinHandle<i32> for exit code
observability_surfaces:
  - Each stdout line delivered as String to mpsc channel — receiver collects verbatim agent output
  - Exit code returned as JoinHandle<i32> join value — non-zero signals failure
  - SendError on dropped receiver is silently ignored (not an error — receiver intentionally dropped on TUI close)
duration: ~5 minutes
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T02: Implement `launch_agent_streaming` in `assay-core::pipeline`

**Replaced the T01 `todo!()` stub with a real `launch_agent_streaming` implementation that spawns a child process, streams stdout line-by-line through an mpsc channel, and returns a `JoinHandle<i32>` carrying the exit code.**

## What Happened

Replaced the stub body (single `todo!()` line) with the full implementation per the task plan:

1. Assert `cli_args` is non-empty with a clear panic message.
2. Spawn the child process with `Stdio::piped()` for stdout and `Stdio::inherit()` for stderr.
3. Take the piped stdout handle before moving `child` into the background thread.
4. Spawn a `std::thread` that:
   - Wraps stdout in `BufReader` and iterates `lines()`
   - Sends each `Ok(line)` to `line_tx`; breaks on `SendError` (receiver dropped)
   - Breaks on IO errors
   - Calls `child.wait()` and returns the exit code (`status.code().unwrap_or(-1)`; returns `-1` on wait error)
5. The existing `launch_agent()` function was completely untouched.

The `mpsc` import was already present in the file, so no new imports were needed.

## Verification

- `cargo test -p assay-core --test pipeline_streaming` — **3/3 pass**:
  - `streaming_delivers_lines_to_receiver` ✓
  - `streaming_join_handle_returns_exit_code` ✓
  - `streaming_failed_process_returns_nonzero` ✓
- `cargo test -p assay-core` — **all existing tests pass, 0 failures**
- `git diff crates/assay-core/src/pipeline.rs | grep '^-.*fn launch_agent'` — empty (existing function untouched)

## Diagnostics

- Lines are delivered verbatim to the mpsc channel — receiver collects full agent stdout
- Exit code surfaced via `handle.join().unwrap()` — non-zero means process failed
- Dropped receiver (TUI closed) causes silent loop break — not logged (intentional, not an error)
- Spawn failures panic immediately with "failed to spawn agent subprocess" — surfaced in stderr

## Deviations

None. Implementation matched the task plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/pipeline.rs` — replaced `todo!()` stub with real `launch_agent_streaming` implementation
