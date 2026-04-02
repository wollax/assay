# S05: Job Monitoring, Timeout & Graceful Shutdown — Research

**Date:** 2026-03-17

## Summary

S05 adds three capabilities to Smelt: (1) a `smelt status` CLI command that shows live progress for a running job, (2) timeout enforcement that kills containers exceeding their session/job timeouts, and (3) Ctrl+C signal handling that triggers graceful container teardown with no orphans.

The codebase is well-positioned for this. Tokio already has `signal` (with the `signal` feature enabled in `Cargo.toml`) and `tokio::time::timeout` available. The `DockerProvider::teardown()` method is battle-tested — called explicitly in both success and error paths per D023/D026. The main work is: wrapping the `exec` call in `execute_run()` with `tokio::select!` for timeout + signal handling, adding a `JobMonitor` struct that writes state to a well-known file, and adding a `smelt status` subcommand that reads it.

The recommended approach is a lightweight file-based monitoring model: `smelt run` writes a JSON state file (e.g., `.smelt/run-state.json`) that `smelt status` reads. This avoids IPC complexity (Unix sockets, shared memory) and is sufficient for the single-job case in M001. Timeout enforcement wraps the assay exec future in `tokio::select!` against `tokio::time::sleep`. Signal handling uses `tokio::signal::ctrl_c()` in another `tokio::select!` branch.

## Recommendation

**Three modules, one integration point:**

1. **`monitor.rs`** in smelt-core — `JobMonitor` struct that tracks job state (active session names, container ID, start time, phase). Writes JSON to `.smelt/run-state.json` on state transitions. Cleans up the file on teardown.

2. **`smelt status` subcommand** in smelt-cli — Reads `.smelt/run-state.json`, prints formatted status (job name, elapsed time, active container, current phase). Returns exit code 1 if no job is running.

3. **Timeout + signal handling in `execute_run()`** — Replace the bare `provider.exec()` call with a `tokio::select!` on three branches: exec completion, timeout expiry (`tokio::time::sleep`), and Ctrl+C (`tokio::signal::ctrl_c()`). Both timeout and signal branches call `provider.teardown()` before returning. The timeout value comes from the max session timeout in the manifest (already computed by `AssayInvoker::build_run_command()`).

This approach keeps changes minimal. The `execute_run()` function in `run.rs` is the single integration point — it already has the "async block + unconditional teardown" pattern (D026) that just needs the `select!` wrapper added.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Ctrl+C signal handling | `tokio::signal::ctrl_c()` | Already in deps (`signal` feature enabled). Cross-platform, async-native. |
| Timeout enforcement | `tokio::time::timeout()` or `tokio::time::sleep` in `select!` | Standard tokio pattern. More flexible than `timeout()` when combined with signal handling. |
| State file serialization | `serde_json` or reuse existing `serde` + `toml` | `serde` already in deps. TOML or JSON both work — JSON is simpler for ephemeral state. |
| Container health check | `bollard::Docker::inspect_container()` | Already used in tests (`InspectContainerOptions`). Returns container state, health, running status. |

## Existing Code and Patterns

- `crates/smelt-cli/src/commands/run.rs` → `execute_run()` is the orchestration hub. The async-block-plus-unconditional-teardown pattern (D026) is the insertion point for timeout/signal handling. The `result` async block (lines ~55-100) needs to be wrapped in `tokio::select!`.
- `crates/smelt-core/src/docker.rs` → `DockerProvider::teardown()` handles stop (10s grace) + force remove. Already tolerant of "already stopped" (304) and "not found" (404). Safe to call from signal/timeout handlers.
- `crates/smelt-core/src/provider.rs` → `RuntimeProvider` trait. No changes needed — `teardown()` is the only cleanup method required.
- `crates/smelt-core/src/assay.rs` → `AssayInvoker::build_run_command()` already computes max timeout across sessions. Extract this logic for timeout enforcement.
- `crates/smelt-cli/src/main.rs` → `Cli` enum has only `Run`. Add `Status` variant here.
- `crates/smelt-cli/src/commands/mod.rs` → Add `pub mod status;`.
- `crates/smelt-core/src/config.rs` → `SmeltConfig` has `default_timeout: u64`. Could be used as a fallback job-level timeout.
- `crates/smelt-cli/tests/docker_lifecycle.rs` → `test_manifest_with_repo()` helper and `docker_provider_or_skip()` pattern for integration tests.

## Constraints

- **Tokio features already enabled:** `macros`, `rt-multi-thread`, `process`, `signal` — all needed features are present in workspace `Cargo.toml`. No dependency changes needed for core signal/timeout work.
- **No `serde_json` in workspace deps.** For JSON state files, either add `serde_json` or use TOML (already available). TOML is fine for a simple state file and avoids a new dep.
- **Single-job model.** M001 doesn't support concurrent jobs. The state file can be a single file, not a directory of job-specific files.
- **`DockerProvider::exec()` blocks until stream completes.** The exec method consumes the entire output stream synchronously (via `while let Some(chunk) = output.next().await`). To support timeout/cancellation, the exec future must be cancellable — wrapping it in `tokio::select!` will drop the future on the other branch winning, which closes the stream. This is safe because teardown force-removes the container anyway.
- **Rust 2024 edition** — `async fn` in traits (RPITIT) is used per D019. New trait methods should follow this pattern.
- **D023/D026 pattern:** Explicit teardown in both success and error paths. The signal handler must also call teardown — not rely on Drop.

## Common Pitfalls

- **Double teardown on Ctrl+C** — If signal fires during the exec phase, the `select!` branch calls teardown. But the unconditional teardown after the async block will also try to call teardown. `DockerProvider::teardown()` already handles 404 (container not found) gracefully, so double teardown is safe. Just make sure the second teardown failure is logged, not propagated.
- **Signal handler registration timing** — `tokio::signal::ctrl_c()` must be polled (awaited or selected on) before the signal arrives. Register the future *before* starting the exec, not after. In `select!`, both branches start immediately, so this is natural.
- **State file left behind on crash** — If Smelt crashes (panic, OOM, SIGKILL), the state file won't be cleaned up. `smelt status` should handle stale state files by checking if the PID recorded in the file is still alive (or just showing the state with a "may be stale" warning).
- **Timeout granularity** — Session-level timeouts vs job-level timeout. The manifest has per-session `timeout` but no job-level timeout. For M001, use the max session timeout as the job timeout (matching `AssayInvoker::build_run_command()` behavior). A separate `[job].timeout` field can be added later.
- **Stream cancellation safety** — Dropping a bollard exec output stream mid-read is safe — the container continues running (it's just the output pipe that's broken). The subsequent `teardown()` call will stop and remove the container properly.

## Open Risks

- **`smelt status` freshness:** The file-based approach means `smelt status` shows a snapshot. If `smelt run` updates infrequently, status can appear stale. Mitigation: update the state file at each phase transition (provision, write manifest, exec start, exec complete, collecting, teardown) and include a timestamp.
- **PID-based liveness detection:** Using `std::process::id()` to record the runner PID works on Unix but is a weak signal (PID reuse). For M001 this is acceptable — it's a development tool, not a production system.
- **Container health beyond "running":** `bollard::inspect_container` gives container state (running/exited/OOMKilled). The monitor could poll this periodically to detect container crashes independent of exec stream failure. Whether to add periodic health polling or rely solely on exec stream failure is a design choice — exec stream failure is simpler and sufficient for M001.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / Tokio | — | No specific skill found in available_skills; standard tokio patterns suffice |
| Docker / bollard | — | No specific skill found; bollard API is straightforward |

No relevant skills were found in `<available_skills>` for this work. The technologies involved (tokio signal handling, bollard container inspection) are well-documented in their respective crate docs and don't require specialized agent skills.

## Sources

- Tokio signal feature already enabled in `Cargo.toml`: `tokio = { version = "1", features = ["macros", "rt-multi-thread", "process", "signal"] }`
- `tokio::signal::ctrl_c()` is the standard async Ctrl+C handler (source: tokio crate docs at `target/doc/tokio/signal/fn.ctrl_c.html`)
- `tokio::select!` macro enables racing multiple async branches — standard pattern for timeout + cancellation
- `bollard::Docker::inspect_container()` already used in test suite for container state verification (source: `crates/smelt-cli/tests/docker_lifecycle.rs`)
- D023/D026 establish explicit teardown pattern that signal handling must follow (source: `.kata/DECISIONS.md`)
