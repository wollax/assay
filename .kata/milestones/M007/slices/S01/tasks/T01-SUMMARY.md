---
id: T01
parent: S01
milestone: M007
provides:
  - "`pub fn launch_agent_streaming(cli_args, working_dir, line_tx) -> JoinHandle<i32>` exported from `assay-core::pipeline`"
  - "Edge case: empty cli_args returns handle resolving to -1 (no panic)"
  - "Integration test `launch_agent_streaming_delivers_all_lines` — real subprocess, asserts ordered lines and exit code 0"
  - "Integration test `launch_agent_streaming_empty_args_returns_minus_one` — edge case guard"
key_files:
  - crates/assay-core/src/pipeline.rs
key_decisions:
  - "Spawn failure (bad binary/path) mapped to -1 exit code via immediate `thread::spawn(|| -1)` — keeps caller API uniform"
  - "Sender is dropped inside the thread after the read loop exits, signaling Disconnected to receivers"
patterns_established:
  - "`BufReader::lines()` + `mpsc::Sender<String>` pattern for stdout streaming; exit code returned via JoinHandle<i32>"
observability_surfaces:
  - "Per-line stream: collect from `Receiver<String>` paired with the sender"
  - "Exit code: `JoinHandle::join().unwrap_or(-1)`; panic maps to -1"
duration: 5min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: Add `launch_agent_streaming` to assay-core pipeline

**Added `launch_agent_streaming` to `assay-core::pipeline`: spawns subprocess, streams stdout line-by-line via `mpsc::Sender<String>`, returns `JoinHandle<i32>` with exit code.**

## What Happened

Added `launch_agent_streaming` directly below `launch_agent` in `crates/assay-core/src/pipeline.rs`. The function:

1. Guards against empty `cli_args` with an immediate `thread::spawn(|| -1)` return.
2. Spawns the child with `stdout: Stdio::piped()`, `stderr: Stdio::null()`.
3. Takes child stdout before moving the child into a background thread.
4. In the background thread: wraps stdout in `BufReader`, iterates `.lines()`, sends each line via `line_tx.send()`. Breaks on send error (receiver dropped) or IO error.
5. Drops the sender (closing the channel) after the loop so the receiver sees `Disconnected`.
6. Calls `child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)` and returns that as the thread result.

Two tests added inline in the `#[cfg(test)]` module:
- `launch_agent_streaming_delivers_all_lines` — real `sh -c 'printf ...'` subprocess, collects via `recv_timeout`, asserts `["line1", "line2"]` in order and exit code 0.
- `launch_agent_streaming_empty_args_returns_minus_one` — guards the edge case path.

Existing `launch_agent` function is completely untouched.

## Verification

```
cargo test -p assay-core -- launch_agent_streaming
# 2 tests: ok (launch_agent_streaming_delivers_all_lines, launch_agent_streaming_empty_args_returns_minus_one)

cargo build -p assay-core
# Finished dev profile — zero warnings

grep -n "pub fn launch_agent\b" crates/assay-core/src/pipeline.rs
# 219:pub fn launch_agent(  — signature unchanged
```

## Diagnostics

- Per-line stream: pair `Receiver<String>` with the `Sender` passed in; loop `recv_timeout` until `Disconnected`.
- Exit code: `handle.join().unwrap_or(-1)`.
- Panic in spawned thread → `handle.join()` returns `Err` → callers map to -1.

## Deviations

None. Implementation exactly matched the plan.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/pipeline.rs` — Added `launch_agent_streaming` function (~50 lines) and two inline tests; existing content unchanged
