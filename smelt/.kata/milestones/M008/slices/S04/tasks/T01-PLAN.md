---
estimated_steps: 5
estimated_files: 6
---

# T01: Add worker_host to QueuedJob, JobStateResponse, and TUI

**Slice:** S04 — Dispatch routing + round-robin + TUI/API worker field
**Milestone:** M008

## Description

Add `worker_host: Option<String>` to the data model (`QueuedJob`), API response (`JobStateResponse`), and TUI table so the field is ready for T02 to populate at dispatch time. All existing tests must continue to pass with `worker_host: None` as the default.

## Steps

1. Add `worker_host: Option<String>` with `#[serde(default)]` to `QueuedJob` in `types.rs`
2. Add `worker_host: Option<String>` to `JobStateResponse` in `http_api.rs`; populate from `QueuedJob.worker_host` in the `From` impl
3. Add a "Worker" column to the TUI table in `tui.rs` — show the host string or "-" when None
4. Update all test helpers that construct `QueuedJob` (in `queue.rs` tests and `tests.rs`) to include `worker_host: None`
5. Add `test_tui_render_worker_host` in `tui.rs` (or `tests.rs`) — construct a job with `worker_host: Some("worker-1".into())`, render via `TestBackend`, assert no panic and "worker-1" appears in output. Add `test_worker_host_in_api_response` — construct a `QueuedJob` with `worker_host: Some(...)`, convert to `JobStateResponse`, assert the field is present.

## Must-Haves

- [ ] `QueuedJob` has `worker_host: Option<String>` with `#[serde(default)]` (backward compat for existing state files)
- [ ] `JobStateResponse` includes `worker_host` field in JSON serialization
- [ ] TUI table has a "Worker" column showing host or "-"
- [ ] All existing tests pass with `worker_host: None`
- [ ] New test: TUI renders worker_host without panic
- [ ] New test: API response includes worker_host

## Verification

- `cargo test --workspace` — all existing tests pass (0 failures)
- `cargo test -p smelt-cli --lib -- tui::tests::test_tui_render_worker_host` — passes
- `cargo test -p smelt-cli --lib -- http_api::tests` or inline test — worker_host in response

## Observability Impact

- Signals added/changed: `worker_host` field added to queue state TOML persistence (via QueuedJob serde)
- How a future agent inspects this: `GET /api/v1/jobs` returns `worker_host` per job; TUI shows Worker column; `.smelt-queue-state.toml` includes `worker_host` field
- Failure state exposed: `worker_host: null` in JSON distinguishes locally-dispatched jobs from remote ones

## Inputs

- `crates/smelt-cli/src/serve/types.rs` — `QueuedJob` struct to extend
- `crates/smelt-cli/src/serve/http_api.rs` — `JobStateResponse` struct and `From` impl to extend
- `crates/smelt-cli/src/serve/tui.rs` — `render()` function to extend with Worker column

## Expected Output

- `crates/smelt-cli/src/serve/types.rs` — `QueuedJob` gains `worker_host: Option<String>`
- `crates/smelt-cli/src/serve/http_api.rs` — `JobStateResponse` gains `worker_host: Option<String>`
- `crates/smelt-cli/src/serve/tui.rs` — TUI table gains Worker column + test
- `crates/smelt-cli/src/serve/queue.rs` — test helpers updated with `worker_host: None`
