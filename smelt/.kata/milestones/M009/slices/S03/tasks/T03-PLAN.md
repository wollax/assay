---
estimated_steps: 6
estimated_files: 8
---

# T03: Decompose serve tests.rs into directory module by feature area

**Slice:** S03 — Large file decomposition
**Milestone:** M009

## Description

Convert `serve/tests.rs` (1370 lines, 31 tests) from a flat file into a `serve/tests/` directory module. Tests are grouped by feature area: queue, dispatch, HTTP API, SSH dispatch, and config. Shared helpers stay in `mod.rs`. The TUI test (small, single test) stays in `mod.rs`.

## Steps

1. Create `crates/smelt-cli/src/serve/tests/` directory. Move `tests.rs` to `tests/mod.rs`.
2. Keep shared helpers in `mod.rs`: `VALID_MANIFEST_TOML`, `manifest()`, `start_test_server()`, and the TUI render test. Add `mod queue; mod dispatch; mod http; mod ssh_dispatch; mod config;` declarations.
3. Extract queue tests (`test_queue_fifo_order`, `test_queue_max_concurrent`, `test_queue_cancel_queued`, `test_queue_retry_eligible`) to `tests/queue.rs`. Add appropriate `use super::*` and `use crate::serve::*` imports.
4. Extract dispatch tests (`test_dispatch_loop_two_jobs_concurrent`, `test_cancellation_broadcast`) and watcher tests (`test_watcher_picks_up_manifest`, `test_watcher_moves_to_dispatched`) to `tests/dispatch.rs`.
5. Extract HTTP API tests (`test_http_post_enqueues_job`, `test_http_post_invalid_toml`, `test_http_get_jobs`, `test_http_get_job_by_id`, `test_http_delete_queued_job`, `test_http_delete_running_job`, `test_serve_http_responds_while_running`) to `tests/http.rs`.
6. Extract SSH dispatch tests (`test_manifest_delivery_and_remote_exec`, `test_round_robin_two_workers`, `test_failover_one_offline`, `test_all_workers_offline_requeue`, `test_worker_host_in_queue_state_roundtrip`, `test_state_sync_round_trip`) to `tests/ssh_dispatch.rs`. Extract config tests (`test_worker_config_roundtrip`, `test_worker_config_defaults`, `test_server_config_no_workers_parses`, `test_worker_config_deny_unknown_fields`, `test_worker_config_empty_host_fails_validation`, `test_worker_config_empty_user_fails_validation`, `test_server_config_roundtrip`, `test_server_config_missing_queue_dir`, `test_server_config_invalid_max_concurrent`) to `tests/config.rs`. Fix all imports. Verify: `cargo test --workspace`.

## Must-Haves

- [ ] `tests/mod.rs` exists and is < 500 lines — contains shared helpers + TUI test + mod declarations
- [ ] `tests/queue.rs` exists with 4 queue tests
- [ ] `tests/dispatch.rs` exists with dispatch + watcher tests
- [ ] `tests/http.rs` exists with 7 HTTP API tests
- [ ] `tests/ssh_dispatch.rs` exists with 6 SSH dispatch tests
- [ ] `tests/config.rs` exists with 9 config tests
- [ ] All 31 serve tests pass — zero regressions
- [ ] `cargo build --workspace` compiles with no new warnings

## Verification

- `cargo test -p smelt-cli` — all tests pass, 0 failures
- `cargo test --workspace` — 286+ pass, 0 failures
- `wc -l crates/smelt-cli/src/serve/tests/mod.rs` — under 500

## Observability Impact

- Signals added/changed: None — pure refactoring
- How a future agent inspects this: `cargo test`, `cargo build`, `wc -l`
- Failure state exposed: Compiler errors on broken imports/visibility

## Inputs

- `crates/smelt-cli/src/serve/tests.rs` — the 1370-line file to decompose
- `crates/smelt-cli/src/serve/mod.rs` — declares `mod tests`
- T02 output — `ssh/mock.rs` module with `MockSshClient` (import path may have changed)

## Expected Output

- `crates/smelt-cli/src/serve/tests/mod.rs` — shared helpers + TUI test + mod declarations (< 500 lines)
- `crates/smelt-cli/src/serve/tests/queue.rs` — 4 queue tests
- `crates/smelt-cli/src/serve/tests/dispatch.rs` — dispatch + watcher tests
- `crates/smelt-cli/src/serve/tests/http.rs` — 7 HTTP API tests
- `crates/smelt-cli/src/serve/tests/ssh_dispatch.rs` — 6 SSH dispatch tests
- `crates/smelt-cli/src/serve/tests/config.rs` — 9 config tests
