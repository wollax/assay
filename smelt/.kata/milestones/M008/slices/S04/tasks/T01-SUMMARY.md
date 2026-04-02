---
id: T01
parent: S04
milestone: M008
provides:
  - worker_host field on QueuedJob (Option<String>, serde-default for backward compat)
  - worker_host field on JobStateResponse (exposed in JSON API)
  - Worker column in TUI table (shows host or "-")
  - test_tui_render_worker_host test
  - test_worker_host_in_api_response and test_worker_host_none_in_api_response tests
key_files:
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/serve/tui.rs
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - "worker_host: Option<String> with #[serde(default)] — None means local dispatch, Some means remote worker"
patterns_established:
  - "worker_host plumbing pattern: types.rs field → http_api.rs From impl → tui.rs column"
observability_surfaces:
  - "GET /api/v1/jobs returns worker_host per job (null for local, string for remote)"
  - "TUI Worker column shows host or '-'"
  - ".smelt-queue-state.toml persists worker_host via QueuedJob serde"
duration: 8min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T01: Add worker_host to QueuedJob, JobStateResponse, and TUI

**Added `worker_host: Option<String>` to data model, API response, and TUI — ready for dispatch routing in T02**

## What Happened

Added the `worker_host` field across the three layers:

1. **types.rs** — `QueuedJob` gained `worker_host: Option<String>` with `#[serde(default)]` for backward compatibility with existing state files that lack the field.
2. **http_api.rs** — `JobStateResponse` gained `worker_host: Option<String>`, populated from `QueuedJob.worker_host.clone()` in the `From` impl. Added test module with two tests covering `Some` and `None` cases.
3. **tui.rs** — Added "Worker" column (16-char width) showing the host string or "-" when None. Added `test_tui_render_worker_host` using `TestBackend` to verify rendering.
4. **queue.rs** + **tests.rs** — Updated all `QueuedJob` construction sites to include `worker_host: None`.

## Verification

- `cargo test --workspace` — 155 passed, 0 failed
- `cargo test -p smelt-cli --lib -- tui::tests::test_tui_render_worker_host` — passed
- `cargo test -p smelt-cli --lib -- http_api::tests` — 2 tests passed (worker_host present and null cases)

## Diagnostics

- `GET /api/v1/jobs` returns `worker_host` per job — `null` for locally-dispatched, string for remote
- TUI shows Worker column with host or "-"
- `.smelt-queue-state.toml` includes `worker_host` field via QueuedJob serde

## Deviations

- `ServerState::new()` takes `usize` not `Option`, fixed in tui test (used `1` instead of `None`)
- Added an extra test `test_worker_host_none_in_api_response` beyond plan — verifies null serialization

## Known Issues

None

## Files Created/Modified

- `crates/smelt-cli/src/serve/types.rs` — Added `worker_host: Option<String>` with `#[serde(default)]` to `QueuedJob`
- `crates/smelt-cli/src/serve/http_api.rs` — Added `worker_host` to `JobStateResponse`, `From` impl, and new test module
- `crates/smelt-cli/src/serve/tui.rs` — Added Worker column to table and `test_tui_render_worker_host` test
- `crates/smelt-cli/src/serve/queue.rs` — Updated `enqueue()` and `make_job()` with `worker_host: None`
- `crates/smelt-cli/src/serve/tests.rs` — Updated `QueuedJob` construction with `worker_host: None`
