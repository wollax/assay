---
id: T01
parent: S03
milestone: M002
provides:
  - exec_streaming<F>() on RuntimeProvider trait and DockerProvider impl
  - Silent exec() — eprint! removed from output loop
key_files:
  - crates/smelt-core/src/provider.rs
  - crates/smelt-core/src/docker.rs
key_decisions:
  - exec_streaming uses RPITIT style matching existing trait methods (D019)
  - output_cb called for both StdOut and StdErr chunks; ExecHandle still carries full buffered output
patterns_established:
  - Callback-based streaming alongside buffered exec — same create/start/inspect skeleton, different output path
observability_surfaces:
  - debug! logs per chunk in exec_streaming (stream="stdout"/"stderr")
  - info! logs at exec_streaming created/started/complete with exec_id and exit_code
  - ExecHandle.stdout / ExecHandle.stderr carry full buffered output for post-hoc inspection
duration: ~10 min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add `exec_streaming()` to `RuntimeProvider` trait and `DockerProvider`

**Added `exec_streaming<F>()` to `RuntimeProvider` trait and `DockerProvider`, and silenced `exec()` by removing its `eprint!` calls.**

## What Happened

1. **`provider.rs`** — Added `exec_streaming<F>()` directly after `exec()` in the `RuntimeProvider` trait, using the RPITIT pattern (`impl Future<Output = ...> + Send`) established by the other trait methods. Bound: `F: FnMut(&str) + Send + 'static`.

2. **`docker.rs`** — Implemented `exec_streaming<F>()` on `DockerProvider` by copying the full `create_exec` → `start_exec` → `inspect_exec` structure from `exec()`. In the output loop, both `StdOut` and `StdErr` arms call `output_cb(&text)` and push to their respective buffers. No `eprint!` — that is the caller's responsibility. `debug!` and `info!` tracing preserved.

3. **`docker.rs`** — Removed both `eprint!("{text}")` calls from the `exec()` output loop (`StdOut` and `StdErr` arms). `exec()` is now fully silent; callers receive output exclusively via `ExecHandle.stdout` / `ExecHandle.stderr`.

## Verification

- `cargo test -p smelt-core` → `test result: ok. 110 passed` (unit tests + doc-tests)
- `cargo build -p smelt-cli` → compiled without errors (trait impl complete, no missing method errors)
- `grep -n "eprint!" crates/smelt-core/src/docker.rs` → no output (zero matches confirmed)

## Diagnostics

- Streaming chunks visible via `RUST_LOG=debug` — each chunk logs `debug!(stream = "stdout"/"stderr", ...)`
- `exec_streaming` start/complete logged at `info!` with `exec_id` and `exit_code`
- `ExecHandle.stdout` / `ExecHandle.stderr` carry full output for any caller needing post-hoc inspection
- Error mid-stream propagates as `Err(SmeltError::provider_with_source("exec_streaming", ...))` — callback not invoked after error

## Deviations

None. Implementation followed the plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/provider.rs` — `exec_streaming<F>()` method added to `RuntimeProvider` trait after `exec()`
- `crates/smelt-core/src/docker.rs` — `exec_streaming<F>()` impl added to `DockerProvider`; `eprint!` removed from `exec()` loop arms
