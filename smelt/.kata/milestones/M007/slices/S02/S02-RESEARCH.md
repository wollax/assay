# S02: Atomic state file — write on every transition — Research

**Date:** 2026-03-23
**Domain:** Rust persistence / atomic file writes / serde TOML
**Confidence:** HIGH

## Summary

S02 adds two free functions to `queue.rs` (or a new `state_file.rs`):
`write_queue_state(queue_dir, jobs)` and `read_queue_state(queue_dir) -> Vec<QueuedJob>`.
`write_queue_state` serializes the in-memory `VecDeque<QueuedJob>` to TOML, writes to a
`.tmp` file, then atomically renames to the target path. Every mutation to `ServerState`
(`enqueue`, `complete`, `cancel`, and the Running transition in `dispatch.rs`) must call
`write_queue_state` after the in-memory change.

S01 delivered everything this slice needs: `QueuedJob` is fully `Serialize + Deserialize`,
all timing fields are `u64`, and `toml` is already a production dependency. The only new
dependency risk is `fs::rename` cross-device portability — but `queue_dir` and its `.tmp`
sibling are on the same filesystem by construction, so rename is always atomic.

The unit test is the critical deliverable: write → read → assert round-trip equality on all
fields, including `manifest_path`, `attempt`, `status`, and the two timing fields. The
round-trip test is the primary proof that S03's `load_or_new()` can reconstruct a
`ServerState` from the file.

## Recommendation

Implement two standalone free functions in `queue.rs` (no new file needed — the module is
small). Follow the `JobMonitor::write()` precedent for the atomic pattern but use
`fs::rename` directly (no temp-path wrapper needed since the `.tmp` lifetime is controlled
inline). Wire calls into the four `ServerState` mutation methods. The public function
signatures are locked by the S02→S03 boundary map and must not be changed.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic file write | `std::fs::rename()` after writing `.tmp` | POSIX-rename is the established pattern in this codebase (`smelt-core/monitor.rs` precedent); avoids partial-write corruption; same FS guaranteed |
| TOML serialization | `toml::to_string_pretty()` from the `toml` crate (already a prod dep) | Already in `smelt-cli` production deps; `RunState` already uses it; no new dependency |
| Tempfile naming | `format!("{}.tmp", target_path.display())` inline | Simpler than `tempfile::NamedTempFile` for this use case; temp file lives beside the target (same FS); no crate overhead |

## Existing Code and Patterns

- `crates/smelt-core/src/monitor.rs` — `JobMonitor::write()` writes TOML via `toml::to_string_pretty()` then `fs::write()`. **Important: this does NOT use atomic rename** — it writes directly. S02 must use rename-into-place (D100 contract). Follow the serialization pattern but add the rename step.
- `crates/smelt-cli/src/serve/queue.rs` — `ServerState` owns the `VecDeque<QueuedJob>` and all four mutation methods (`enqueue`, `try_dispatch`, `complete`, `cancel`). These are the exact four callsites that need `write_queue_state` wired in. Also `dispatch.rs`'s Running transition (`job.started_at = Some(now_epoch())`) is a fifth mutation that needs a write.
- `crates/smelt-cli/src/serve/types.rs` — `QueuedJob` has full `Serialize + Deserialize`; `JobId` uses `#[serde(transparent)]` so it serializes as a bare string; `now_epoch()` and `elapsed_secs_since()` are the canonical time helpers.
- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig.queue_dir: PathBuf` is the base path for the state file: `queue_dir/.smelt-queue-state.toml`. The `.tmp` path is `queue_dir/.smelt-queue-state.toml.tmp`.

## State File Format

The TOML file holds a top-level array of `QueuedJob` entries serialized as an array of
inline tables. Because `JobId` uses `#[serde(transparent)]`, `id` serializes as a string.
`PathBuf` serializes as a UTF-8 string by `toml`. `JobSource` / `JobStatus` serialize as
lowercase snake_case strings via `#[serde(rename_all = "snake_case")]`.

Expected file layout:

```toml
[[jobs]]
id = "job-1"
manifest_path = "/tmp/.tmpXXXXX"
source = "http_api"
attempt = 0
status = "queued"
queued_at = 1742000000
started_at = 1742000001

[[jobs]]
id = "job-2"
manifest_path = "/var/smelt-queue/dispatched/1742000000-my-job.toml"
source = "directory_watch"
attempt = 1
status = "retrying"
queued_at = 1741999900
started_at = 1742000000
```

Deserializing back to `Vec<QueuedJob>` requires a wrapper struct:

```rust
#[derive(Serialize, Deserialize)]
struct QueueState {
    jobs: Vec<QueuedJob>,
}
```

This is the minimal TOML-native approach — TOML arrays of tables use `[[key]]` syntax,
which maps to `Vec` on a struct with a `jobs` field.

## Constraints

- **No `deny_unknown_fields` on `QueuedJob`** (D108) — future fields must be additive without breaking old state files on rollback. This is already correct since `QueuedJob` has no such annotation.
- **Atomic write required** (D100) — write to `.smelt-queue-state.toml.tmp` then `fs::rename()`. Never write directly to `.smelt-queue-state.toml`.
- **`read_queue_state` must tolerate a missing file** — first-run case: return empty vec, no error. Tolerate parse errors with `warn!()` + return empty vec — never panic.
- **`queue_dir` must exist before writing** — `write_queue_state` should call `fs::create_dir_all(&queue_dir)` before writing the `.tmp` file. `ServerState::enqueue()` is the first write and `queue_dir` may not yet exist.
- **`write_queue_state` is called while the `Mutex<ServerState>` lock is held** — the function must not block on I/O for long. Synchronous `fs::write` + `fs::rename` is fine (no async needed; the dispatch loop already runs in tokio but write completes in microseconds).
- **`PathBuf` serializes as a UTF-8 string in TOML** — `toml` crate does this correctly; no special handling needed.
- **`dispatch.rs` Running transition** — `run_job_task` holds the lock briefly to transition `Dispatching → Running` and set `started_at`. This is also a mutation that should write state. However, `run_job_task` currently takes `Arc<Mutex<ServerState>>` not `queue_dir`. The write call needs `queue_dir` passed in or the write can be skipped for the Running transition (it's a nice-to-have; the critical writes are enqueue/complete/cancel). Decision at implementation time — document in DECISIONS.md.

## Where to Wire Calls

| Method / Callsite | File | Write needed? |
|---|---|---|
| `ServerState::enqueue()` | `queue.rs` | **Yes** — job enters queue |
| `ServerState::try_dispatch()` → Dispatching | `queue.rs` | Nice-to-have (Dispatching is transient) |
| `ServerState::complete()` → Retrying/Complete/Failed | `queue.rs` | **Yes** — job reaches terminal or retry state |
| `ServerState::cancel()` | `queue.rs` | **Yes** — job removed from queue |
| `run_job_task` → Running transition | `dispatch.rs` | Optional (see constraint above) |

The three mandatory writes (`enqueue`, `complete`, `cancel`) cover all durable state
transitions. `try_dispatch` → Dispatching is transient (the job will transition to Running
or failed within milliseconds) and can be omitted to keep the critical path tight.

The challenge: `ServerState` methods don't currently have access to `queue_dir`. Two options:
1. **Add `queue_dir: Option<PathBuf>` to `ServerState`** — constructor takes it; write calls
   are self-contained. Cleanest — no signature changes to callers.
2. **Pass `queue_dir` as a parameter to each mutation** — callers must supply it. Invasive.

Option 1 is the correct choice. `ServerState` already knows `max_concurrent`; adding
`queue_dir` is a natural extension. Existing tests that use `ServerState::new(n)` need
minimal changes — either `new` takes `Option<PathBuf>` or a separate `new_with_persistence`
constructor is added. **Do not break existing tests.**

## Common Pitfalls

- **Wrong TOML structure for Vec** — `toml::to_string_pretty()` on a bare `Vec<QueuedJob>` will fail because TOML requires a top-level table. Wrap in `QueueState { jobs: Vec<QueuedJob> }`.
- **Forgetting `create_dir_all`** — `queue_dir` may not exist on the first `enqueue`. Writing the `.tmp` file to a non-existent directory returns `ENOENT`. Always call `fs::create_dir_all` before writing.
- **Rename across filesystems** — `fs::rename` fails with `EXDEV` if src and dst are on different filesystems. This cannot happen if `.tmp` is in the same directory as the target. Using `queue_dir/.smelt-queue-state.toml.tmp` guarantees same-FS.
- **Test isolation** — tests that verify state file contents must use a `TempDir` for `queue_dir` to avoid interference. Do not hardcode `/tmp/smelt-queue-state.toml`.
- **`now_epoch()` in tests gives elapsed ≈ 0** — if the round-trip test checks `queued_at`, it should snapshot the value before write and compare after read (not rely on a specific wall-clock value).

## Open Risks

- The Mutex lock is held during the synchronous `fs::write` + `fs::rename` I/O. For a busy queue on slow storage, this could starve other tokio tasks momentarily. Acceptable for M007 (local filesystem, sub-millisecond writes); revisit if benchmarks show lock contention.
- Error handling: if `write_queue_state` fails (e.g. disk full), the in-memory state is updated but the file is not. Current design: log the error via `warn!()` but don't propagate — letting the serve daemon die on a disk-full error would be worse than continuing with a stale file. This should be documented.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / serde / toml | none needed | Built-in knowledge sufficient |

## Sources

- S01 Summary (`S01-SUMMARY.md`) — confirms `QueuedJob` serde status, `now_epoch()` API, and forward intelligence for S02
- `smelt-core/src/monitor.rs` — `JobMonitor::write()` pattern for TOML serialization (sans atomic rename)
- `crates/smelt-cli/Cargo.toml` — confirms `toml` is already a production dep; no new dependencies required
- D100 (atomic file-move semantics), D108 (no `deny_unknown_fields`), D109 (in-flight re-queue policy), D110 (`u64` timing fields)
