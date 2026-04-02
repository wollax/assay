# S03: Streaming Assay Output — Research

**Date:** 2026-03-17

## Summary

S03 adds `exec_streaming()` to `RuntimeProvider` and `DockerProvider`, then wires Phase 7 of `execute_run()` to use it so `assay run` output reaches the user's terminal in real time rather than only after assay exits.

The good news: `DockerProvider::exec()` already processes bollard output chunks incrementally via `output.next().await` — the transport layer is already streaming. The problem is architectural: (1) the `RuntimeProvider` trait exposes no streaming variant, so callers cannot pass a per-chunk callback; (2) `exec()` conflates two concerns — it both streams chunks to stderr via `eprint!("{text}")` inside the loop AND buffers everything into `stdout_buf`/`stderr_buf` for the `ExecHandle` return value; (3) Phase 7 in `run.rs` calls `exec()` and then re-prints the buffered stdout/stderr after the fact, double-printing all assay output.

The fix is minimal: add `exec_streaming(container, command, output_cb: impl FnMut(&str) + Send) -> Result<ExecHandle>` to the `RuntimeProvider` trait; implement it in `DockerProvider` by refactoring the existing `start_exec` loop to call the callback per chunk instead of eprinting; silence the inline `eprint!` inside the existing `exec()` (setup commands shouldn't write to the user's terminal); and replace Phase 7's `exec()` call with `exec_streaming(&container, &cmd, |chunk| eprint!("{chunk}"))`, removing the redundant post-exec `eprint!` block.

## Recommendation

**Add `exec_streaming()` as a second trait method alongside `exec()`** (D046). Remove the `eprint!` calls from `exec()` in `docker.rs` — setup commands produce little output and their stdout/stderr are already returned on the `ExecHandle` for error reporting. `DockerProvider::exec_streaming()` reuses the exact same bollard `create_exec` → `start_exec` → inspect_exec pattern as `exec()`, adding only the callback invocation per chunk. Phase 7 uses `exec_streaming` with `|chunk| eprint!("{chunk}")` and drops the post-exec output block.

This approach:
- Keeps the existing `exec()` callers (Phase 5.5 setup commands) unchanged — no behavioral change for them
- Eliminates the double-print in Phase 7
- Adds a clean, testable streaming API to the trait
- Does not require new dependencies or a change to bollard usage

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Bollard exec streaming | `DockerProvider::exec()` — `start_exec` already returns `StartExecResults::Attached { output, .. }` with a chunk stream | The streaming infra is already wired; just expose the per-chunk callback |
| Trait method signature style | D019 pattern: RPITIT (`impl Future + Send`) in trait | Already used for all trait methods; use same style for `exec_streaming()` |
| Phase 7 error handling | Existing `exec_future` async block pattern in `run.rs` | Just swap `exec()` → `exec_streaming()` inside the same block; teardown/cancellation wiring is unchanged |
| Phase 5.5 exec callers | `provider.exec(&container, &config_cmd)` pattern | These callers remain unchanged — they use buffered `exec()` which is correct for short setup commands |

## Existing Code and Patterns

- `crates/smelt-core/src/docker.rs` — `exec()` already has the full bollard streaming loop at lines ~159–260; `exec_streaming()` is a near-identical refactor: replace `eprint!("{text}")` with `output_cb(&text)` and remove internal eprintln; retain the inspect_exec call at the end for the exit code
- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait; add `exec_streaming` with `F: FnMut(&str) + Send + 'static` bound; RPITIT requires the future to be `Send`, so the callback also needs `Send`
- `crates/smelt-cli/src/commands/run.rs` Phase 7 — replace `provider.exec()` with `provider.exec_streaming()` passing `|chunk| eprint!("{chunk}")` as the callback; remove the `if !handle.stdout.is_empty()` / `if !handle.stderr.is_empty()` eprint block that follows (output already went to terminal via callback)
- `crates/smelt-cli/tests/docker_lifecycle.rs` — add `test_exec_streaming_delivers_chunks_in_order`: provision container, run `printf 'a\nb\nc\n'`, collect chunks via callback into a `Vec<String>`, assert order and content — verifies callback is called per chunk; also assert `ExecHandle.stdout` is populated (streaming variant still buffers for diagnostics)

### Double-print issue (current state)

In `docker.rs` `exec()`, lines ~213–221:
```rust
Ok(LogOutput::StdOut { message }) => {
    let text = String::from_utf8_lossy(&message);
    eprint!("{text}");           // ← printed here (chunk 1)
    stdout_buf.push_str(&text);
}
Ok(LogOutput::StdErr { message }) => {
    let text = String::from_utf8_lossy(&message);
    eprint!("{text}");           // ← printed here (chunk 1)
    stderr_buf.push_str(&text);
}
```

Then in `run.rs` Phase 7, after `exec()` returns:
```rust
if !handle.stdout.is_empty() {
    eprint!("{}", handle.stdout);  // ← printed again (chunk 2 = full buffer)
}
if !handle.stderr.is_empty() {
    eprint!("{}", handle.stderr);  // ← printed again (chunk 2 = full buffer)
}
```

Every line of assay output appears twice. The fix: remove `eprint!` from `exec()` (make it silent), add `exec_streaming()` with callback, Phase 7 uses `exec_streaming()`.

### Current exec() in docker.rs already streams

The existing bollard `start_exec` returns a chunk stream. The `while let Some(chunk) = output.next().await` loop processes each chunk as bollard delivers it — no buffering until completion. The full stdout/stderr are only held in `stdout_buf`/`stderr_buf` for the ExecHandle return. `exec_streaming()` can be implemented by extracting the loop body and calling the callback instead of (or in addition to) eprintln.

## Constraints

- **D019 (firm):** RPITIT style for trait methods; `exec_streaming<F>(..., output_cb: F) -> impl Future<Output = crate::Result<ExecHandle>> + Send where F: FnMut(&str) + Send + 'static` — the `'static` bound may be needed depending on how the future captures `output_cb`; verify during implementation (if RPITIT captures `&mut F` lifetimes, `'static` may not be needed)
- **D046 (firm):** `exec_streaming()` added alongside `exec()`; buffered `exec()` retained for Phase 5.5 setup commands — do NOT replace `exec()` entirely
- **Bollard callback across async boundary:** `FnMut` callbacks are not `Send` by default; since the bollard stream is polled with `.await`, the callback must be `Send` to satisfy the future's Send bound; use `F: FnMut(&str) + Send` or `F: FnMut(&str) + Send + 'static` depending on lifetime requirements
- **ExecHandle stdout/stderr fields:** `exec_streaming()` should still populate `stdout` and `stderr` on the returned `ExecHandle` — callers need the full output for error reporting (e.g., Phase 7 includes `handle.stderr.trim()` in the `anyhow::bail!` message on non-zero exit)
- **Existing exec() callers unchanged:** Phases 5.5 setup callers in `run.rs` and all tests that call `provider.exec()` must compile and behave identically after S03 — only the `eprint!` removal is a behavioral change (setup output goes silent, which is correct)

## Common Pitfalls

- **FnMut + Send lifetime conflict** — If `exec_streaming` takes `output_cb: F` where `F: FnMut(&str) + Send`, and the RPITIT future captures `F`, the future's `Send` requires `F: Send`. Use `F: FnMut(&str) + Send + 'static` or make the lifetime explicit. If `'static` is unacceptable (e.g., caller captures a local Vec), try `F: FnMut(&str) + Send + '_` or restructure to pass chunks via a `Vec<String>` accumulator inside the future and return them alongside the ExecHandle. The simplest working solution: use an `Arc<Mutex<Vec<String>>>` for the test accumulator.

- **Forgetting to remove post-exec eprint in Phase 7** — After switching to `exec_streaming()`, the existing `if !handle.stdout.is_empty() { eprint!("{}", handle.stdout); }` block must be deleted. Leaving it in causes double-printing to resume.

- **Removing eprint from exec() breaks test assertions** — Some tests assert `handle.stdout.contains(...)` — these are not affected by removing `eprint!`. No tests assert on what was printed to stderr during exec. Removal is safe.

- **exec_streaming exit code handling** — The exit code comes from `inspect_exec()` after the stream is consumed, just like `exec()`. Don't return early before calling `inspect_exec()` even if the callback returns early or errors.

- **Stdout vs stderr in callback** — Assay writes its progress output to stderr. The callback should be called for both stdout and stderr chunks. Use a single `output_cb` parameter for all output, or provide two separate callbacks (`stdout_cb`, `stderr_cb`). Single callback is simpler and matches the S03 boundary spec (`stdout_cb: impl FnMut(&str)`); both stdout and stderr chunks should call it so assay's stderr progress messages stream to the user.

- **TTY mode** — `attach_tty` in `CreateExecOptions` defaults to `false`; don't set it to `true` for streaming. With TTY, stdout and stderr are merged and the output is not multiplexed — the bollard `LogOutput` variants would collapse. Keep TTY off to preserve `StdOut`/`StdErr` distinction.

## Open Risks

- **FnMut + Send trait bound complexity with RPITIT** — Rust's RPITIT feature requires `Send` futures; a mutable closure that captures state (e.g., a `Vec` to accumulate chunks in tests) may not be `Send` if it holds non-Send state. This is a known friction point. Mitigation: implement the test collector as an `Arc<Mutex<Vec<String>>>` shared between the callback and the assertion code — `Arc<Mutex<T>>` is `Send` when `T: Send`.

- **exec() eprint removal may change test output** — Removing `eprint!` from `exec()` means setup command output (config write, mkdir, spec writes) no longer prints to test output. Tests that relied on `--nocapture` to see intermediate output for debugging will see less. This is acceptable — the important diagnostics (Phase 5.5 progress messages via `eprintln!`) are in `run.rs`, not in the DockerProvider.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / Tokio | — | none found (standard) |
| bollard | — | none found (project-specific) |

## Sources

- `DockerProvider::exec()` streaming loop — existing `start_exec` + `while let Some(chunk) = output.next().await` pattern (source: `crates/smelt-core/src/docker.rs` lines 159–260)
- `RuntimeProvider` trait — RPITIT method signatures; D019 pattern (source: `crates/smelt-core/src/provider.rs`)
- Phase 7 exec + double-print issue (source: `crates/smelt-cli/src/commands/run.rs` lines 227–250)
- D046 decision: `exec_streaming()` alongside `exec()`; streaming takes callback (source: `.kata/DECISIONS.md`)
- S02 Forward Intelligence: Phase 7 currently uses buffered exec; streaming replacement should handle ExecHandle return semantics (source: `.kata/milestones/M002/slices/S02/S02-SUMMARY.md`)
- Bollard `StartExecResults::Attached` stream type (source: `~/.cargo/registry/src/*/bollard-*/src/exec.rs`)
