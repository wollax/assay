# S02: Directory Watch + HTTP API — Research

**Researched:** 2026-03-23
**Domain:** Axum HTTP API, directory polling, in-process job dispatch integration
**Confidence:** HIGH

## Summary

S02 builds the two job ingress paths for `smelt serve`: a polling directory watcher and an axum REST API. Both call into the `JobQueue` + `ServerState` types that S01 delivers, and the HTTP API additionally exposes live job state as JSON. The implementation is straightforward — the interesting work is in the integration with S01's types and the `run_with_cancellation()` adapter.

The main technical challenge is bridging `run_with_cancellation()`'s generic `cancel: F` parameter (expects `Future<Output = std::io::Result<()>>`) to `CancellationToken::cancelled()` (returns `Future<Output = ()>`). S01's `run_job_task()` must wrap or adapt this. S02 calls `run_job_task()` from its `dispatch_loop` and should treat the bridging as S01's problem — but if S01 didn't fully solve it, S02 must handle it. This is the single highest-risk dependency.

The directory watcher uses `std::fs::read_dir` polling at a 2-second interval — no new crate needed. `axum` 0.8 is not yet in the lockfile but has perfect version alignment with what is already transitively present (hyper 1.8, tower 0.5, tower-http 0.6, tokio 1.50). `serde_json` needs to be added to `smelt-cli/Cargo.toml` for JSON response serialization. `tokio-util` 0.7 is already a transitive dep but needs the `"rt"` feature explicitly declared to access `CancellationToken`.

## Recommendation

**Structure S02 in three layers:**

1. **`serve/queue_watcher.rs`** — `DirectoryWatcher` struct: `std::fs::read_dir` polling on a `tokio::time::interval(Duration::from_secs(2))`; moves matched `.toml` files to `queue_dir/dispatched/<ts>-<name>.toml` via `std::fs::rename` (atomic on same filesystem); calls `JobQueue::enqueue()`. No `notify` crate — polling is the spec and has zero dependencies.

2. **`serve/http_api.rs`** — axum router with 4 routes; `Arc<Mutex<ServerState>>` shared via axum `State` extractor; `JobStateResponse` is a serde type that mirrors `QueuedJob` with computed fields (`elapsed_secs`, `started_at_secs`). Plain `application/json` responses via `axum::Json`.

3. **`serve/dispatch_loop.rs`** — already owned by S01; S02 wires it to the two ingress sources. The dispatch loop owns the `tokio::spawn` per job and passes a child `CancellationToken` to each `run_job_task()`.

**Integration test strategy:** use `axum::serve` in a `tokio::spawn` within the test; send real HTTP requests via `reqwest` (already a transitive dep via octocrab) or write raw requests via `tokio::net::TcpStream`. For directory watcher tests, write a real TOML file to a `TempDir`, wait 3s, assert job appeared in state.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic file move for pickup | `std::fs::rename` | Already in std; atomic on same filesystem; the spec's "file-move semantics" (D100) is exactly this |
| HTTP routing | `axum` 0.8 (add to smelt-cli/Cargo.toml) | Already transitively present via kube (hyper 1.8 + tower 0.5 + tower-http 0.6); zero new transitive deps |
| JSON serialization for responses | `serde_json` (add to smelt-cli/Cargo.toml) | Already in smelt-core; transitively present; just needs explicit dep in smelt-cli |
| CancellationToken | `tokio-util = { version = "0.7", features = ["rt"] }` | Already in lockfile as transitive dep; just needs `features = ["rt"]` explicitly in workspace |
| Broadcast cancel across N jobs | `CancellationToken::child_token()` | One parent token per `smelt serve` session; each job gets a cloned child; cancel parent → all children fired |

## Existing Code and Patterns

- `crates/smelt-cli/src/commands/run.rs::run_with_cancellation()` — The function S01's `run_job_task()` wraps. Takes `cancel: F where F: Future<Output = std::io::Result<()>> + Send`. For `CancellationToken`, the adapter is: `async { token.cancelled().await; Ok(()) }`. This single line bridges the type mismatch. If S01 doesn't already provide this bridge, S02 must add it to `run_job_task()`.
- `crates/smelt-cli/src/commands/run.rs::RunArgs` — Must be constructable from a manifest path alone (dry_run=false, no_pr=false). `run_job_task()` in S01 likely constructs it internally. S02 only passes the `PathBuf`.
- `crates/smelt-core/src/monitor.rs::JobMonitor + RunState` — Used by `run_with_cancellation()` for per-job state files at `.smelt/runs/<job-name>/state.toml`. `smelt status <job>` reads these. No changes needed here.
- `crates/smelt-core/src/manifest.rs::JobManifest` — S02's HTTP POST handler receives raw TOML body and calls `JobManifest::load()` (or `from_str()`). Need to check if `JobManifest::from_str()` exists or if we need to add one (currently `load()` reads from `PathBuf`). The HTTP ingress path can write the TOML body to a temp file and call `load()`, or a `from_str()` helper that takes TOML text + a dummy path for error messages.
- `crates/smelt-cli/src/commands/watch.rs` — Pattern for testability via generic `F: ForgeClient`. HTTP API handlers follow the same pattern: accept `State<Arc<Mutex<ServerState>>>` and return `impl IntoResponse`. No test doubles needed for the axum layer — use `axum::serve` in test with a real bound port.

## Constraints

- **`run_with_cancellation()` is in `smelt-cli` (binary crate)** — it's `pub` but lives in the binary. S02 calls it indirectly via S01's `run_job_task()`. If S01 moved `run_job_task` or `run_with_cancellation()` into `smelt-core`, verify the location before calling.
- **`axum` not yet a direct dep in `smelt-cli/Cargo.toml`** — must be added. Version 0.8 aligns perfectly with the existing transitive graph (hyper 1.8, tower 0.5, tower-http 0.6, tokio 1.50). No version conflicts expected.
- **`serde_json` not in `smelt-cli/Cargo.toml`** — add `serde_json = "1"` as a production dep.
- **`tokio-util` needs `features = ["rt"]` for `CancellationToken`** — add to workspace `[workspace.dependencies]` with that feature, then reference from `smelt-cli/Cargo.toml`. Currently the lockfile has tokio-util 0.7.18 without the rt feature declared in the workspace.
- **`JobManifest` has no `from_str()` method** — `load()` reads from a `PathBuf`. HTTP API handler options: (a) write body to a `NamedTempFile` and call `load()`, or (b) add `JobManifest::from_str(toml: &str, source_path: &Path) -> Result<Self>` helper to smelt-core. Option (b) is cleaner and should be added in S02.
- **Atomic rename for watcher only works on the same filesystem** — `queue_dir` and `queue_dir/dispatched/` are on the same filesystem by definition. Fine.
- **`smelt-cli` is a binary crate** — no `missing_docs` lint enforcement. `serve/` module structure can use `pub(crate)` freely without doc comments.
- **File-move semantics (D100)** — manifest is moved to `dispatched/` before enqueue, so double-pickup on restart is impossible for files already moved. Files not yet moved (power failure between write and rename) will be re-picked up. This is the documented behavior.

## Common Pitfalls

- **`CancellationToken::cancelled()` returns `Future<Output=()>` not `Future<Output=std::io::Result<()>>`** — The adapter `async { token.cancelled().await; Ok(()) }` is required. Forget this and the type won't unify.
- **`std::fs::rename` across filesystems panics or errors** — Not a real concern since `dispatched/` is always a subdirectory of `queue_dir`, but create `queue_dir/dispatched/` with `std::fs::create_dir_all` before any rename attempt to avoid "No such file" errors.
- **axum `State` extractor requires `Clone` on the state** — `Arc<Mutex<ServerState>>` is `Clone`. Don't wrap in a newtype unless it also derives `Clone`.
- **HTTP DELETE returning 409 for a running job** — check `JobStatus::Running | JobStatus::Dispatching` to distinguish from `Queued`. The 409 must be returned before any state mutation.
- **`JobManifest::validate()` errors in the HTTP POST handler** — The handler should call `manifest.validate()` after parsing and return a 422 with the error text if validation fails, not a 500. Validation errors are client errors.
- **Directory watcher interval drift** — Use `tokio::time::interval` (which accounts for elapsed time) not `tokio::time::sleep` in a loop (which drifts). This is a minor correctness issue but makes the polling interval more predictable.
- **`Mutex` poisoning on panic in a tokio task** — if `run_job_task()` panics while holding the lock, the `ServerState` mutex becomes poisoned and all subsequent state reads fail. Use `Result::unwrap_or_else` or call `.unwrap()` with the understanding that panic = bug. Alternatively use `parking_lot::Mutex` which doesn't poison, but that's a new dep — just document the invariant.
- **Test port conflicts** — when binding axum in integration tests, use `TcpListener::bind("127.0.0.1:0")` to get an OS-assigned port, then read back the actual port from `local_addr()`. Never hardcode a port in tests.

## Open Risks

- **S01 may not have been implemented yet** — The roadmap marks S01 as `[x]` but the git log shows no S01 commit, and the branch is `kata/root/M006/S02`. Either S01 was done in a different way (types planned but not in codebase), or S02 must implement both S01's types AND its own in one pass. **If S01's types don't exist in the codebase when S02 begins, S02 must implement them first.** Check for `serve/` module in `smelt-cli/src/` before starting.
- **`run_with_cancellation()` API mismatch with CancellationToken** — the generic future parameter was designed for the oneshot pattern (D037). The `CancellationToken` adapter is a one-liner but if S01 introduced a different interface (e.g. a new `run_job(manifest, cancel_token)` function), use that instead.
- **`JobManifest::from_str()` doesn't exist** — either add it in S02 or use tempfile approach. The from_str approach is cleaner but requires a smelt-core change. Pre-decide this during planning to avoid mid-task discovery.
- **axum version pinning** — `axum = "0.8"` is compatible with the transitive graph but is not yet in the lockfile. Cargo will resolve it on the next `cargo build`. If the resolution pulls in a different hyper or tower version, the build will fail. Run `cargo build` after adding the dep as the first verification step.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| axum HTTP framework | n/a | none found (covered by docs) |
| tokio-util CancellationToken | n/a | none found (covered by tokio docs) |

## New Dependencies Required

Add to `smelt-cli/Cargo.toml`:
```toml
axum = "0.8"
serde_json = "1"
tokio-util = { version = "0.7", features = ["rt"] }
```

Add to `Cargo.toml` workspace `[workspace.dependencies]`:
```toml
tokio-util = { version = "0.7", features = ["rt"] }
serde_json = "1"
```

Note: `uuid` is NOT needed — the roadmap says `JobId` format is `<job-name>-<uuid-v4>`. However `uuid` is not in the lockfile. Alternatives: use `std::time` + counter for uniqueness (simpler, no new dep), or add `uuid = { version = "1", features = ["v4"] }`. Recommend the time+counter approach unless S01 already chose uuid.

## Sources

- Code analysis: `smelt-cli/src/commands/run.rs`, `smelt-core/src/monitor.rs`, `smelt-core/src/manifest.rs` (direct read)
- Lockfile analysis: `Cargo.lock` (transitive dep graph — hyper 1.8, tower 0.5.3, tower-http 0.6.8, tokio-util 0.7.18 all confirmed present)
- Roadmap boundary map: S01→S02 interface contracts (inline in M006-ROADMAP.md)
- Decisions: D098–D102 (serve architecture, CancellationToken, file-move semantics, axum choice)
