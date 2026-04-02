---
estimated_steps: 4
estimated_files: 2
---

# T01: Add `exec_streaming()` to `RuntimeProvider` trait and `DockerProvider`

**Slice:** S03 — Streaming Assay Output
**Milestone:** M002

## Description

The bollard streaming loop already exists inside `DockerProvider::exec()` — this task exposes it as a clean callback-based API. `exec_streaming<F>()` is added to the `RuntimeProvider` trait alongside the existing `exec()`. `DockerProvider` implements it by reusing the exact same `create_exec` → `start_exec` → `inspect_exec` skeleton, calling `output_cb(&text)` per chunk instead of `eprint!`. The inline `eprint!` calls are removed from `exec()` — setup command output was never meant to appear on the user's terminal; it's available on `ExecHandle` for error reporting.

## Steps

1. **Add `exec_streaming` to `RuntimeProvider` trait** (`provider.rs`): Add the method with RPITIT style (D019):
   ```rust
   fn exec_streaming<F>(
       &self,
       container: &ContainerId,
       command: &[String],
       output_cb: F,
   ) -> impl std::future::Future<Output = crate::Result<ExecHandle>> + Send
   where
       F: FnMut(&str) + Send + 'static;
   ```
   Place it directly after `exec()` in the trait body. No default implementation.

2. **Implement `exec_streaming` on `DockerProvider`** (`docker.rs`): Copy the full `create_exec` → `start_exec` loop → `inspect_exec` structure from `exec()`. In the `while let Some(chunk) = output.next().await` loop, for both `StdOut` and `StdErr` arms, call `output_cb(&text)` and push to `stdout_buf`/`stderr_buf`. Remove the `eprint!` call from these arms (there is no `eprint!` in `exec_streaming` — that is the caller's responsibility). Retain `debug!` logging. Return `ExecHandle` with populated stdout/stderr exactly as `exec()` does.

3. **Remove `eprint!` from `exec()`** (`docker.rs`): Delete the `eprint!("{text}");` lines in both `StdOut` and `StdErr` match arms of `exec()`'s output loop. The `debug!` logging and buffer pushes remain. `exec()` becomes fully silent — callers receive output on the `ExecHandle`.

4. **Verify compilation and tests** (`smelt-core`): Run `cargo test -p smelt-core`. All existing unit tests (parse_memory_bytes, parse_cpu_nanocpus) must pass. Confirm `grep -n "eprint!" crates/smelt-core/src/docker.rs` returns no lines.

## Must-Haves

- [ ] `exec_streaming<F>()` is present on the `RuntimeProvider` trait with `F: FnMut(&str) + Send + 'static` bound
- [ ] `DockerProvider::exec_streaming()` calls `output_cb(&text)` for every `StdOut` and `StdErr` chunk
- [ ] `DockerProvider::exec_streaming()` still populates `ExecHandle.stdout` and `ExecHandle.stderr`
- [ ] `DockerProvider::exec_streaming()` calls `inspect_exec()` after the stream completes (exit code retrieval)
- [ ] `DockerProvider::exec()` has no `eprint!` calls — both arms silenced
- [ ] `cargo test -p smelt-core` passes clean

## Verification

- `cargo test -p smelt-core` → `test result: ok.`
- `grep -n "eprint!" crates/smelt-core/src/docker.rs` → no output (zero matches)
- `cargo build -p smelt-core` compiles without error
- `cargo build -p smelt-cli` compiles without error (trait impl is complete, no missing method errors)

## Observability Impact

- Signals added/changed: `exec()` is now silent — setup command output (config write, specs dir, spec files) no longer reaches stderr. Phase 5.5 diagnostic `eprintln!` calls in `run.rs` (e.g. "Writing assay config...") remain unaffected — those are in the caller, not in DockerProvider.
- How a future agent inspects this: `handle.stdout` / `handle.stderr` on the returned `ExecHandle` carry full buffered output for any `exec()` call; error paths in Phase 5.5 already include `handle.stderr.trim()` in their messages.
- Failure state exposed: if `exec_streaming()` fails mid-stream, the `Err(e)` from bollard propagates via `return Err(SmeltError::provider_with_source(...))` exactly as in `exec()`; the callback is not called after the error.

## Inputs

- `crates/smelt-core/src/docker.rs` — existing `exec()` with bollard streaming loop (lines ~159–260); `exec_streaming()` reuses this exact structure
- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait; D019 RPITIT method pattern already established by `provision`, `exec`, `collect`, `teardown`
- D046 decision: `exec_streaming()` alongside `exec()`; buffered exec retained for setup

## Expected Output

- `crates/smelt-core/src/provider.rs` — `exec_streaming<F>()` method added to trait after `exec()`
- `crates/smelt-core/src/docker.rs` — `exec_streaming<F>()` impl added; `eprint!` removed from `exec()` loop arms
