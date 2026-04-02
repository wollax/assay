---
estimated_steps: 4
estimated_files: 2
---

# T03: End-to-end integration tests for round-robin, failover, and worker_host visibility

**Slice:** S04 — Dispatch routing + round-robin + TUI/API worker field
**Milestone:** M008

## Description

Write integration tests that exercise the full pipeline: enqueue jobs → dispatch_loop routes via MockSshClient → verify round-robin distribution, offline-worker failover, all-offline re-queue, and worker_host visibility in `QueuedJob` state. These tests use real `ServerState` and `dispatch_loop` with `MockSshClient` — no subprocess spawning, no real SSH.

## Steps

1. Write `test_round_robin_two_workers`: Create 2 `WorkerConfig` entries and a `MockSshClient` pre-loaded with alternating probe(Ok) + scp(Ok) + exec(Ok exit 0) + scp_from(Ok) results for 4 jobs. Create a `ServerState`, enqueue 4 jobs. Run `dispatch_loop` with a short cancellation timeout (enough for 2 dispatch ticks). Assert: jobs alternate `worker_host` between worker 0 and worker 1 (round-robin pattern).
2. Write `test_failover_one_offline`: 2 workers, worker 0 probe always fails (Err), worker 1 probe always succeeds. Enqueue 2 jobs. Run dispatch. Assert: both jobs have `worker_host` == worker 1's host.
3. Write `test_all_workers_offline_requeue`: 2 workers, both probes fail. Enqueue 1 job. Run one dispatch tick. Assert: job status is `Queued` (re-queued), `worker_host` is `None`.
4. Write `test_worker_host_in_queue_state_roundtrip`: Create a `QueuedJob` with `worker_host: Some("w1.example.com")`, serialize to TOML via `write_queue_state`, read back via `read_queue_state`, assert `worker_host` is preserved.

## Must-Haves

- [ ] `test_round_robin_two_workers` proves jobs alternate between 2 workers
- [ ] `test_failover_one_offline` proves all jobs route to the surviving worker
- [ ] `test_all_workers_offline_requeue` proves job returns to Queued when no workers respond
- [ ] `test_worker_host_in_queue_state_roundtrip` proves worker_host survives TOML persistence
- [ ] All tests use MockSshClient — no real SSH connections

## Verification

- `cargo test -p smelt-cli --lib -- serve::tests::test_round_robin_two_workers` — passes
- `cargo test -p smelt-cli --lib -- serve::tests::test_failover_one_offline` — passes
- `cargo test -p smelt-cli --lib -- serve::tests::test_all_workers_offline_requeue` — passes
- `cargo test -p smelt-cli --lib -- serve::tests::test_worker_host_in_queue_state_roundtrip` — passes
- `cargo test --workspace` — all pass

## Observability Impact

- Signals added/changed: None — tests verify existing signals
- How a future agent inspects this: Run the named test functions to verify dispatch routing behavior
- Failure state exposed: Test assertions include descriptive messages for each check

## Inputs

- `crates/smelt-cli/src/serve/dispatch.rs` — `dispatch_loop`, `select_worker` from T02
- `crates/smelt-cli/src/serve/ssh.rs` — `MockSshClient` with probe/exec/scp/scp_from result queues
- `crates/smelt-cli/src/serve/queue.rs` — `ServerState`, `write_queue_state`, `read_queue_state`
- `crates/smelt-cli/src/serve/types.rs` — `QueuedJob` with `worker_host` from T01

## Expected Output

- `crates/smelt-cli/src/serve/tests.rs` — 4 new integration tests proving round-robin, failover, re-queue, and persistence
