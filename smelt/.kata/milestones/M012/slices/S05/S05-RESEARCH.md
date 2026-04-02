# S05: Dispatch Integration, State Backend Passthrough & Final Assembly — Research

**Date:** 2026-03-28

## Summary

S05 is the terminal integration slice for M012. It must wire `GithubTrackerSource` (S03) and `LinearTrackerSource` (S04) into the `smelt serve` main loop as a `TrackerPoller` background task, pass `state_backend` from `JobManifest` through to the Assay `RunManifest` TOML inside containers, update the TUI to show tracker-sourced job metadata, update `examples/server.toml` and `README.md`, and prove the end-to-end flow via integration tests.

The codebase is well-structured for this integration. The `dispatch_loop` in `serve/dispatch.rs` already runs under `tokio::select!` alongside `DirectoryWatcher::watch()` and the HTTP server. The `TrackerPoller` can be added as another arm of the `tokio::select!` in `serve.rs::execute()`, or as a concurrent task spawned before the select. The `TrackerSource` trait, `issue_to_manifest()` free function, and `ServerState::enqueue()` provide the complete pipeline: poll → transition(ready→queued) → issue_to_manifest → write temp TOML → enqueue → dispatch_loop picks it up.

The `state_backend` passthrough requires extending `AssayInvoker::build_run_manifest_toml()` to serialize `manifest.state_backend` into the TOML string. The `SmeltRunManifest` struct needs a `state_backend: Option<StateBackendConfig>` field, and `StateBackendConfig` already has `Serialize` + `Deserialize` derives.

## Recommendation

Build in four tasks:
1. **TrackerPoller** — new struct in `serve/tracker_poller.rs` (or extend `tracker.rs`) with a polling loop. Construct the correct `TrackerSource` impl from config. Call `ensure_labels()` once, then poll on interval.
2. **State backend passthrough** — add `state_backend` to `SmeltRunManifest`, serialize it in `build_run_manifest_toml()`. Add unit tests.
3. **TUI + serve wiring** — add `TrackerPoller` to the `tokio::select!` in `serve.rs::execute()`. Update TUI to show `Source` column (replacing or extending `Worker` column). Update `examples/server.toml` and `README.md`.
4. **Integration test** — end-to-end test using `MockTrackerSource` proving the full flow: poller picks up issue, transitions state, enqueues job, dispatch_loop processes it.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Polling interval | `tokio::time::interval()` | Already used in `dispatch_loop` and `DirectoryWatcher` with `MissedTickBehavior::Skip` |
| Temp file for manifest | `tempfile::NamedTempFile` + `std::mem::forget(TempPath)` | D105 pattern already used in `http_api.rs` for POST handler — exact same use case |
| Tracker source selection | Match on `config.tracker.provider` string | S03/S04 provide the two impls; match dispatch at construction time |
| Cancellation | `CancellationToken` child token | D099 pattern — TrackerPoller gets a child of the serve-level token |

## Existing Code and Patterns

- `crates/smelt-cli/src/serve/dispatch.rs` — `dispatch_loop()` is generic over `SshClient`; TrackerPoller runs alongside it, feeding jobs into `ServerState::enqueue()`. The poller does NOT go through `dispatch_loop` — it uses the same `ServerState` directly.
- `crates/smelt-cli/src/serve/queue_watcher.rs` — `DirectoryWatcher::watch()` is the exact pattern for TrackerPoller: poll, parse/validate, `state.lock().unwrap().enqueue(path, source)`. Key difference: tracker writes a temp file (D105 pattern), DirectoryWatcher reads from disk.
- `crates/smelt-cli/src/serve/tracker.rs` — `TrackerSource` trait, `issue_to_manifest()` free function (D161), `load_template_manifest()`, `MockTrackerSource`. The poller calls these.
- `crates/smelt-cli/src/serve/github/source.rs` — `GithubTrackerSource<G: GhClient>` with `ensure_labels()` (takes `&self`).
- `crates/smelt-cli/src/serve/linear/source.rs` — `LinearTrackerSource<L: LinearClient>` with `ensure_labels()` (takes `&mut self`). Note: **mutable** self required because it populates the label cache `HashMap`.
- `crates/smelt-cli/src/serve/config.rs` — `TrackerConfig` already validated at startup. `ServerConfig::load()` calls `load_template_manifest()` at startup (D017).
- `crates/smelt-cli/src/serve/types.rs` — `JobSource::Tracker` already exists.
- `crates/smelt-cli/src/serve/tui.rs` — Renders 6 columns: Job ID, Manifest, Status, Attempt, Elapsed, Worker. Needs a `Source` column to distinguish `Tracker` from `HttpApi`/`DirectoryWatch`.
- `crates/smelt-core/src/assay.rs` — `SmeltRunManifest` is `pub(crate)` with `deny_unknown_fields`. Needs `state_backend` field. `build_run_manifest_toml()` builds the TOML string — passthrough goes here.
- `crates/smelt-core/src/tracker.rs` — `StateBackendConfig` enum with `Serialize + Deserialize`.
- `crates/smelt-cli/src/commands/serve.rs` — `execute()` function with `tokio::select!` running dispatch_loop, watcher, HTTP, ctrl_c, TUI poll. TrackerPoller becomes a sixth arm.
- `crates/smelt-cli/src/serve/http_api.rs` — POST handler uses `tempfile::NamedTempFile` + `std::mem::forget(TempPath)` pattern (D105) — TrackerPoller should use the same pattern for writing generated manifests to disk.

## Constraints

- **`TrackerSource` is not object-safe** — RPITIT async methods make `dyn TrackerSource` impossible. The poller must use a concrete type or an enum dispatcher (like `AnyProvider` in D084).
- **`LinearTrackerSource::ensure_labels()` takes `&mut self`** — the label cache requires mutability. The poller must hold a mutable reference at startup. After `ensure_labels()`, all trait methods take `&self`.
- **`SmeltRunManifest` uses `deny_unknown_fields`** — adding `state_backend` field must use `#[serde(default, skip_serializing_if = "Option::is_none")]` to maintain backward compat with existing manifests that don't have it.
- **`state_backend` in Assay's RunManifest lives at the top level** — it's a `[state_backend]` TOML section. The `SmeltRunManifest` struct models the run manifest Assay reads, so the field goes directly on `SmeltRunManifest`.
- **Template manifest is loaded once at startup** — `ServerConfig::load()` validates and loads it. The poller should receive the pre-loaded `JobManifest` (template) at construction time, not re-load it on every poll.
- **Temp file lifetime (D105)** — `std::mem::forget(TempPath)` leaks the file descriptor. For a long-running daemon, this means temp files accumulate. Acceptable per D105 for the dispatch queue since manifests are consumed by `dispatch_loop`. Same applies to tracker-generated manifests.
- **TUI renders in a `std::thread`** — brief lock on `ServerState` to clone job data. Adding a `Source` column means including `job.source` in the cloned tuple.

## Common Pitfalls

- **Forgetting `ensure_labels()` before `transition_state()`** — `LinearTrackerSource` will error with "label not in cache" if `ensure_labels()` is not called first. The poller MUST call it once at startup. `GithubTrackerSource::ensure_labels()` is idempotent and safe to retry but should also be called once.
- **Non-atomic Linear label transition (D170)** — `remove_label` then `add_label` as two mutations. If `add_label` fails, the issue is in label limbo. The poller should log a warning and continue — the issue will be picked up again on the next poll cycle since it won't have `smelt:ready` anymore.
- **Double dispatch prevention (D157)** — The poller MUST call `transition_state(Ready → Queued)` BEFORE `enqueue()`. If the transition fails, skip the issue. This is the atomic guard.
- **`deny_unknown_fields` on `SmeltRunManifest`** — Adding `state_backend` without `#[serde(default)]` would break existing TOML parsing in tests. Must be `Option<StateBackendConfig>` with `default` + `skip_serializing_if`.
- **TrackerPoller and `tokio::select!` ordering** — The poller is a sixth arm. If it panics, the select exits and the server shuts down. Wrap the poller body in a catch-all error handler that logs and continues.

## Open Risks

- **`TrackerSource` concrete dispatch** — Since the trait is not object-safe, the poller needs either an enum wrapper (like `AnyProvider` D084) or must be generic. An enum wrapper `AnyTrackerSource { GitHub(GithubTrackerSource<SubprocessGhClient>), Linear(LinearTrackerSource<ReqwestLinearClient>) }` implementing the poll/transition loop manually is likely simplest.
- **Template manifest `state_backend` field** — The template manifest may or may not have a `[state_backend]` section. `issue_to_manifest()` clones the template, which already includes `state_backend: Option<StateBackendConfig>`. It flows through naturally — no special handling needed.
- **README scope** — How much tracker documentation to add to README. Suggest a focused section on `[tracker]` config with one GitHub and one Linear example, plus a `[state_backend]` example in the job manifest documentation.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust/Tokio | N/A | Core language — no skill needed |
| ratatui | N/A | Already used extensively — patterns established |

No external technology skills needed — this is pure internal integration work.

## Sources

- S03-SUMMARY.md — `GithubTrackerSource` patterns, `GhClient` trait, `ensure_labels()` idempotent via `--force`
- S04-SUMMARY.md — `LinearTrackerSource` patterns, `ensure_labels()` with find-or-create + cache, two-mutation transitions
- D084 — `AnyProvider` enum pattern for non-object-safe trait dispatch
- D098 — In-process dispatch (tracker jobs go through same `dispatch_loop`)
- D105 — `std::mem::forget(TempPath)` for temp manifest files
- D150 — Periodic polling, not webhooks
- D151 — One tracker source per `smelt serve` instance
- D154 — State backend passthrough via Smelt-side serde struct
- D157 — Double-dispatch prevention via label transition before enqueue
- D161 — `issue_to_manifest()` is a free function, not a trait method
- D162 — Template must have zero sessions
