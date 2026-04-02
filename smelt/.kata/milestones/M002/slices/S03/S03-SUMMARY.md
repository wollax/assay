---
id: S03
parent: M002
milestone: M002
provides:
  - exec_streaming<F>() on RuntimeProvider trait and DockerProvider impl
  - Silent exec() — eprint! removed; output available exclusively via ExecHandle
  - Phase 7 of execute_run() uses exec_streaming() — live chunk delivery, no double-print
  - test_exec_streaming_delivers_chunks_in_order integration test in docker_lifecycle.rs
requires:
  - slice: S02
    provides: Working execute_run() with Phase 5.5 wired; DockerProvider::exec() bollard loop to extract
affects:
  - S04
key_files:
  - crates/smelt-core/src/provider.rs
  - crates/smelt-core/src/docker.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - D046: exec_streaming() added alongside exec(); buffered exec() retained for setup phases
  - D049: output_cb bound is FnMut(&str) + Send + 'static; Arc<Mutex<Vec<String>>> satisfies this in tests
patterns_established:
  - Callback-based streaming alongside buffered exec — same create/start/inspect skeleton, different output path
  - Arc<Mutex<Vec<String>>> for chunk accumulation in async test callbacks (satisfies Send + 'static)
  - Teardown-before-assert ordering in streaming test (D039 pattern)
observability_surfaces:
  - debug! logs per chunk in exec_streaming (stream="stdout"/"stderr", text)
  - info! logs at exec_streaming created/started/complete with exec_id and exit_code
  - ExecHandle.stdout / ExecHandle.stderr carry full buffered output for post-hoc inspection
  - test_exec_streaming_delivers_chunks_in_order prints chunk[i] = ... and handle.stdout = ... with --nocapture
  - assay run output streams to stderr in real time via eprint!("{chunk}") in Phase 7 callback
drill_down_paths:
  - .kata/milestones/M002/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M002/slices/S03/tasks/T02-SUMMARY.md
duration: ~20 min total (T01: ~10 min, T02: ~10 min)
verification_result: passed
completed_at: 2026-03-17
---

# S03: Streaming Assay Output

**`exec_streaming<F>()` added to `RuntimeProvider`/`DockerProvider`; Phase 7 wired to live callback delivery; double-print bug eliminated; streaming integration test passes against real Docker.**

## What Happened

**T01** added `exec_streaming<F>()` to the `RuntimeProvider` trait and `DockerProvider` implementation, and silenced the `exec()` output loop. The new method copies the `create_exec` → `start_exec` → `inspect_exec` skeleton from `exec()` but routes each chunk through `output_cb(&text)` instead of `eprint!`. Both `StdOut` and `StdErr` chunks invoke the callback and push to `stdout_buf`/`stderr_buf` so the returned `ExecHandle` still carries full buffered output for diagnostics. The `eprint!` calls were removed from `exec()`'s output loop — `exec()` is now fully silent and available-via-ExecHandle only.

**T02** wired Phase 7 of `execute_run()` to use `exec_streaming()` with `|chunk| eprint!("{chunk}")` as the callback, eliminating the root cause of the double-print bug (the previous pattern: buffered `exec()` + post-exec re-print of `handle.stdout`/`handle.stderr`). The post-exec eprint block was deleted. The `handle.exit_code != 0` bail path was left intact — `exec_streaming()` still populates `ExecHandle.stderr` for the error message. A new integration test `test_exec_streaming_delivers_chunks_in_order` was added to `docker_lifecycle.rs`; it provisions an `alpine:3` container, runs `printf 'a\nb\nc\n'`, collects chunks into an `Arc<Mutex<Vec<String>>>`, tears down, and asserts chunk order and `ExecHandle` population.

## Verification

```
# Full workspace
cargo test --workspace → 7 suites, all "test result: ok." — no FAILED

# Streaming integration test with --nocapture
cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order -- --nocapture
chunk[0] = "a\nb\nc\n"
handle.stdout = "a\nb\nc\n"
test test_exec_streaming_delivers_chunks_in_order ... ok
test result: ok. 1 passed

# eprint! removed from exec()
grep -n "eprint!" crates/smelt-core/src/docker.rs → no matches (exit 1)

# smelt-core unit tests
cargo test -p smelt-core → test result: ok. 110 passed
```

## Requirements Advanced

No `.kata/REQUIREMENTS.md` exists — operating in legacy compatibility mode. Milestone M002 success criteria advanced:

- "Gate output from inside the container is visible on the terminal as `assay run` produces it (streaming, not buffered until exit)" — **fully addressed** by this slice.

## Requirements Validated

- Streaming exec delivers output incrementally — **validated** by `test_exec_streaming_delivers_chunks_in_order` against a real Docker container with real bollard chunk delivery.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

None. Implementation followed the plan exactly.

## Known Limitations

- Streaming chunks may arrive in a single combined chunk (e.g. `"a\nb\nc\n"`) rather than three separate chunks when the command completes quickly — this is correct bollard behavior (no artificial per-line splitting). The test asserts joined output equals `"a\nb\nc\n"` rather than asserting exactly 3 chunks.
- `exec()` is now silent — any caller that previously relied on exec() printing output to stderr (for debugging short setup commands) will need to inspect `ExecHandle.stdout`/`ExecHandle.stderr` manually or switch to `exec_streaming()`.

## Follow-ups

- S04: Exit code 2 from `assay run` should surface as `JobPhase::GatesFailed` (or equivalent), not as a generic error — this distinction is not yet implemented.
- S04: `ResultCollector::collect()` behavior against Assay's post-merge state should be verified by unit test.

## Files Created/Modified

- `crates/smelt-core/src/provider.rs` — `exec_streaming<F>()` method added to `RuntimeProvider` trait after `exec()`
- `crates/smelt-core/src/docker.rs` — `exec_streaming<F>()` impl added to `DockerProvider`; `eprint!` removed from `exec()` loop arms
- `crates/smelt-cli/src/commands/run.rs` — Phase 7 uses `exec_streaming()`; post-exec eprint block removed
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `test_exec_streaming_delivers_chunks_in_order` added; `Arc<Mutex>` import added

## Forward Intelligence

### What the next slice should know
- `exec_streaming()` populates both `ExecHandle.stdout` and `ExecHandle.stderr` — S04's exit-code-2 handling can rely on `handle.stderr.trim()` being available for diagnostic messages, same as the existing non-zero exit bail path.
- Phase 7's callback is a plain `|chunk| eprint!("{chunk}")` — S04 can wrap this or replace it with a more sophisticated handler that also accumulates for exit-code-2 message formatting if needed.
- `exec()` is now silent: setup phase commands (provision, teardown, spec writes) produce no terminal output unless the caller explicitly inspects `ExecHandle`. This is intentional.

### What's fragile
- The `'static` bound on `output_cb` prevents using stack-local closures (e.g. `&mut String` accumulator in a test). Any caller needing non-`'static` callbacks must use `Arc<Mutex<T>>` or a channel. See D049.
- `bollard` multiplexes stdout and stderr in the same stream — `exec_streaming()` calls the callback for both. If S04 needs to distinguish stdout vs stderr chunks (e.g. for structured log parsing), a new variant with two callbacks or a tagged enum would be needed.

### Authoritative diagnostics
- `RUST_LOG=debug cargo test ... -- --nocapture` — logs `debug!(stream = "stdout"/"stderr", ...)` per chunk in `exec_streaming`; confirms chunk delivery order and content.
- `handle.stdout` / `handle.stderr` on the returned `ExecHandle` — always populated by `exec_streaming`; reliable for post-hoc inspection even after the callback has fired.

### What assumptions changed
- Original assumption: bollard delivers output line-by-line → Actual: bollard delivers chunks of arbitrary size (may combine multiple lines in one chunk, or split a line across chunks). The test and implementation handle this correctly — no per-line splitting logic exists.
