---
estimated_steps: 4
estimated_files: 2
---

# T02: Implement `launch_agent_streaming` in `assay-core::pipeline`

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

Add `launch_agent_streaming` as a new public free function in `crates/assay-core/src/pipeline.rs`. The function spawns a child process with piped stdout, reads stdout line-by-line in a background thread, sends each line to the provided `mpsc::Sender<String>`, and returns a `JoinHandle<i32>` whose join value is the process exit code. The existing batch `launch_agent()` function is not touched in any way (D108).

This task is proven independently of the TUI, making the streaming primitive verifiable before the event loop refactor in T03.

## Steps

1. In `crates/assay-core/src/pipeline.rs`, add the `launch_agent_streaming` function after the existing `launch_agent()` function:
   ```rust
   pub fn launch_agent_streaming(
       cli_args: &[String],
       working_dir: &Path,
       line_tx: mpsc::Sender<String>,
   ) -> std::thread::JoinHandle<i32> {
       // Spawn child with piped stdout; stderr goes to inherit (visible in terminal) or null
       let mut child = std::process::Command::new(cli_args[0].as_str())
           .args(&cli_args[1..])
           .current_dir(working_dir)
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::inherit())
           .spawn()
           .expect("failed to spawn agent subprocess");
       
       let stdout = child.stdout.take().expect("stdout was piped");
       
       std::thread::spawn(move || {
           use std::io::BufRead;
           let reader = std::io::BufReader::new(stdout);
           for line in reader.lines() {
               match line {
                   Ok(l) => {
                       if line_tx.send(l).is_err() {
                           break; // receiver dropped — TUI closed
                       }
                   }
                   Err(_) => break,
               }
           }
           // Wait for child and return exit code
           match child.wait() {
               Ok(status) => status.code().unwrap_or(-1),
               Err(_) => -1,
           }
       })
   }
   ```
   Note: If `cli_args` could be empty, guard with an early check. Use the existing `std::sync::mpsc` import that's already in the file.

2. Ensure the function is exported from `assay-core` lib root if needed (check `crates/assay-core/src/lib.rs` for `pub use pipeline::*` or add a specific re-export).

3. Handle the case where `cli_args` is empty: the function should panic with a clear message (acceptable — callers always provide at least the binary name).

4. Run `cargo test -p assay-core --test pipeline_streaming` — all three tests from T01 should now pass.

## Must-Haves

- [ ] `launch_agent_streaming` is `pub` in `assay-core::pipeline`
- [ ] Function accepts `cli_args: &[String]`, `working_dir: &Path`, `line_tx: mpsc::Sender<String>`
- [ ] Returns `std::thread::JoinHandle<i32>`
- [ ] Background thread reads stdout line-by-line via `BufReader::lines()`
- [ ] Each line sent to `line_tx`; send errors (receiver dropped) break the loop cleanly
- [ ] Thread returns exit code as join value; on wait error returns -1
- [ ] Existing `launch_agent()` function is completely untouched (zero diff on that function)
- [ ] `cargo test -p assay-core --test pipeline_streaming` — all 3 tests green
- [ ] `cargo test -p assay-core` — all existing tests still pass (no regressions)

## Verification

- `cargo test -p assay-core --test pipeline_streaming` — 3/3 pass
- `cargo test -p assay-core 2>&1 | tail -3` — "test result: ok." with 0 failures
- `git diff crates/assay-core/src/pipeline.rs | grep '^-.*fn launch_agent'` — should be empty (existing function unchanged)

## Observability Impact

- Signals added/changed: `launch_agent_streaming` is itself an observability primitive — it exposes agent stdout as a stream of `String` events. Each line delivered to `line_tx` is visible in whatever UI or log sink receives it. Exit code delivered via join handle is the final status signal.
- How a future agent inspects this: `let handle = launch_agent_streaming(...); for line in line_rx { inspect(line); } let exit = handle.join().unwrap();`
- Failure state exposed: non-zero exit code returned by `handle.join()` is the failure signal; `SendError` on closed receiver is silent (not an error — receiver intentionally dropped on TUI close)

## Inputs

- `crates/assay-core/src/pipeline.rs` — existing file with `launch_agent()` batch impl as structural reference; `std::sync::mpsc` already imported
- `crates/assay-core/tests/pipeline_streaming.rs` — T01 test file whose three tests this task must make pass

## Expected Output

- `crates/assay-core/src/pipeline.rs` — new `launch_agent_streaming` function added; existing code unmodified
- All three tests in `pipeline_streaming.rs` pass
