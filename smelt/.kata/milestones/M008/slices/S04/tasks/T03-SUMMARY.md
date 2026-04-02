---
id: T03
parent: S04
milestone: M008
provides:
  - test_round_robin_two_workers — proves 4 jobs alternate worker_host between 2 workers
  - test_failover_one_offline — proves all jobs route to surviving worker when one is offline
  - test_all_workers_offline_requeue — proves job reverts to Queued with worker_host None
  - test_worker_host_in_queue_state_roundtrip — proves worker_host survives TOML persistence
key_files:
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - "test_all_workers_offline_requeue simulates one dispatch cycle directly rather than running dispatch_loop — avoids infinite re-queue loop when all workers are always offline"
patterns_established:
  - "Integration test pattern: create MockSshClient with pre-loaded result queues, spawn dispatch_loop, poll for terminal status, assert worker_host distribution"
observability_surfaces:
  - none — tests verify existing signals
duration: 10min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T03: End-to-end integration tests for round-robin, failover, and worker_host visibility

**4 integration tests proving round-robin distribution, single-worker failover, all-offline re-queue, and worker_host TOML persistence**

## What Happened

Added 4 tests to `crates/smelt-cli/src/serve/tests.rs`:

1. `test_round_robin_two_workers` — enqueues 4 jobs with 2 mock workers, runs dispatch_loop, asserts jobs alternate worker_host between worker-a and worker-b.
2. `test_failover_one_offline` — 2 workers, worker-a probe always fails, asserts all jobs route to worker-b.
3. `test_all_workers_offline_requeue` — simulates one dispatch cycle with both probes failing, asserts job status reverts to Queued with worker_host None.
4. `test_worker_host_in_queue_state_roundtrip` — creates a QueuedJob with worker_host set, serializes to TOML via write_queue_state, reads back via read_queue_state, asserts worker_host preserved.

All tests use MockSshClient — no real SSH connections.

## Verification

- `cargo test -p smelt-cli --lib -- serve::tests::test_round_robin_two_workers` — passed
- `cargo test -p smelt-cli --lib -- serve::tests::test_failover_one_offline` — passed
- `cargo test -p smelt-cli --lib -- serve::tests::test_all_workers_offline_requeue` — passed
- `cargo test -p smelt-cli --lib -- serve::tests::test_worker_host_in_queue_state_roundtrip` — passed
- `cargo test --workspace` — 155 passed, 0 failed

Slice-level verification:
- `cargo test -p smelt-cli --lib -- dispatch::tests` — 4 passed ✓
- `cargo test -p smelt-cli --lib -- serve::tests::test_round_robin` — passed ✓
- `cargo test -p smelt-cli --lib -- serve::tests::test_failover` — passed ✓
- `cargo test -p smelt-cli --lib -- serve::tests::test_all_workers_offline` — passed ✓
- `cargo test -p smelt-cli --lib -- serve::tests::test_worker_host` — passed ✓ (covers both API test from T01 and persistence roundtrip)
- `cargo test --workspace` — 155 passed, 0 failures ✓
- `grep -c 'allow(dead_code)' crates/smelt-cli/src/serve/config.rs` — verified returns 1 (only retry_backoff_secs) ✓

## Diagnostics

Run the named test functions to verify dispatch routing behavior. Test assertions include descriptive messages for each check.

## Deviations

- `test_all_workers_offline_requeue` simulates one dispatch cycle directly (try_dispatch → select_worker → re-queue) rather than running the full dispatch_loop. Running dispatch_loop with permanently-offline workers would cause an infinite re-queue loop since the job is immediately re-queued and re-dispatched each tick. The direct simulation matches the test in dispatch::tests::test_requeue_all_workers_offline but lives in serve::tests for integration coverage.
- Added `test_worker_host_in_queue_state_roundtrip` (not in original plan) to verify TOML persistence survives round-trip — this was listed in the task plan's must-haves.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/tests.rs` — Added 4 integration tests for round-robin, failover, re-queue, and worker_host persistence
