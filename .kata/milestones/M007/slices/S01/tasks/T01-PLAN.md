---
estimated_steps: 4
estimated_files: 2
---

# T01: Add `launch_agent_streaming` to assay-core pipeline

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

Add a new free function `launch_agent_streaming` to `assay-core::pipeline` that spawns a child process, reads its stdout line-by-line via `BufReader::lines()`, and sends each line to a `mpsc::Sender<String>`. The function returns a `JoinHandle<i32>` whose join value is the child's exit code (-1 on panic). The existing `launch_agent()` batch function is left completely untouched. An integration test using a real echo subprocess proves all lines are delivered in order and the exit code is correct.

This is the streaming foundation that the TUI event loop (T02/T03) will build on. The interface is intentionally simple: callers get a sender for lines and a handle to join when the agent finishes — no blocking, no polling.

## Steps

1. In `crates/assay-core/src/pipeline.rs`, add `pub fn launch_agent_streaming` below `launch_agent`. Signature: `pub fn launch_agent_streaming(cli_args: &[String], working_dir: &Path, line_tx: std::sync::mpsc::Sender<String>) -> std::thread::JoinHandle<i32>`. Implementation: spawn child with `Command::new(&cli_args[0]).args(&cli_args[1..]).current_dir(working_dir).stdout(Stdio::piped()).stderr(Stdio::null())`, then `std::thread::spawn(move || { /* BufReader::lines loop */ })`. In the spawned thread: take child stdout into `BufReader`, iterate `.lines()`, `let _ = line_tx.send(line)` for each. After the loop, call `child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)`. Return the `JoinHandle<i32>`.

2. Handle the edge case where `cli_args` is empty: if empty, immediately `std::thread::spawn(|| -1)` (no process to spawn). This prevents an index-out-of-bounds panic and keeps the caller's error handling consistent.

3. Write an integration test function `launch_agent_streaming_delivers_all_lines` in `crates/assay-core/src/pipeline.rs` (under `#[cfg(test)]`) or in a test module. The test: create an unbounded channel, call `launch_agent_streaming(&["sh", "-c", "printf 'line1\nline2\n'; exit 0"].map(String::from).to_vec(), &std::env::current_dir().unwrap(), tx)`, collect lines from `rx` until `RecvTimeout` or `Disconnected`, join the handle, assert lines == `["line1", "line2"]` and exit code == 0. Use `rx.recv_timeout(Duration::from_secs(5))` in a loop.

4. Run `cargo test -p assay-core` to confirm the new test passes and zero existing tests regress.

## Must-Haves

- [ ] `pub fn launch_agent_streaming(cli_args: &[String], working_dir: &Path, line_tx: Sender<String>) -> JoinHandle<i32>` is exported from `assay-core::pipeline`
- [ ] Integration test `launch_agent_streaming_delivers_all_lines` passes: receives `["line1", "line2"]` in order, join value == 0
- [ ] Existing `launch_agent` function is unchanged (no signature or behavior modification)
- [ ] `cargo test -p assay-core` passes with zero regressions

## Verification

- `cargo test -p assay-core` — full suite green including new streaming test
- Grep confirms `launch_agent` signature unchanged: `grep -n "pub fn launch_agent\b" crates/assay-core/src/pipeline.rs`
- `cargo build -p assay-core` — clean compile, no warnings

## Observability Impact

- Signals added/changed: `launch_agent_streaming` is a new free function — no existing signals changed. The returned `JoinHandle<i32>` carries the exit code as the sole observable outcome; the `Sender<String>` is the per-line observable stream.
- How a future agent inspects this: join the returned handle to get exit code; collect from the receiver to get lines. Test proves both signals are reliable.
- Failure state exposed: panic in the spawned thread → `JoinHandle::join()` returns `Err` → callers map to exit code -1. Empty `cli_args` → immediately returns handle with -1.

## Inputs

- `crates/assay-core/src/pipeline.rs` — existing `launch_agent` and `HarnessWriter` type are the reference for patterns and imports (`std::process::Command`, `Stdio::piped()`, `std::thread::spawn`, `mpsc`)
- D108 — `launch_agent_streaming` as new free function, existing batch function untouched

## Expected Output

- `crates/assay-core/src/pipeline.rs` — `launch_agent_streaming` function added (~25 lines); existing content unchanged
- Integration test green; `cargo test -p assay-core` exits 0
