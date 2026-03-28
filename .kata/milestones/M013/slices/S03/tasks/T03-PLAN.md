---
estimated_steps: 4
estimated_files: 3
---

# T03: Instrument pipeline and merge code paths

**Slice:** S03 — OTel Metrics
**Milestone:** M013

## Description

Place the five metric recording calls at their instrumentation sites in `pipeline.rs` and `merge_runner.rs`. Add `Instant`-based latency measurement for the two histogram metrics. All calls are unconditional (no `cfg` guards) — the recording functions no-op when metrics are not initialized.

## Steps

1. In `pipeline.rs::run_session`, add `crate::telemetry::record_session_launched()` at function entry (after the `#[instrument]` macro, first line of body).
2. In `pipeline.rs::execute_session`, inside the `gate_evaluate` info_span scope: add `crate::telemetry::record_gate_evaluated()` at entry. Capture `let gate_start = Instant::now();` at scope entry. After the gate evaluate logic completes (before the scope's return), add `crate::telemetry::record_gate_eval_latency(gate_start.elapsed().as_secs_f64() * 1000.0)`.
3. In `pipeline.rs`, wrap the `launch_agent` call with timing: `let agent_start = Instant::now();` before the call, and `crate::telemetry::record_agent_run_duration(agent_start.elapsed().as_secs_f64() * 1000.0)` after it returns (regardless of success/failure).
4. In `merge_runner.rs::merge_completed_sessions`, add `crate::telemetry::record_merges_attempted()` at function entry (after the `let start = Instant::now();` line).
5. Run `just ready` to verify zero regression across all 1516+ tests.

## Must-Haves

- [ ] `record_session_launched()` called at `run_session` entry
- [ ] `record_gate_evaluated()` + `record_gate_eval_latency(ms)` called in gate_evaluate scope
- [ ] `record_agent_run_duration(ms)` wraps `launch_agent` call with Instant timing
- [ ] `record_merges_attempted()` called at `merge_completed_sessions` entry
- [ ] `just ready` green — all existing tests pass, no regressions

## Verification

- `just ready` — all tests pass
- `cargo build -p assay-core` — default build clean (no new deps)
- `grep -n 'record_session_launched\|record_gate_evaluated\|record_merges_attempted\|record_gate_eval_latency\|record_agent_run_duration' crates/assay-core/src/pipeline.rs crates/assay-core/src/orchestrate/merge_runner.rs` — confirms 5 call sites across 2 files

## Observability Impact

- Signals added/changed: Five OTel metrics now emitted during pipeline execution (counters increment, histograms record latency in ms)
- How a future agent inspects this: When running with `--features telemetry` and an OTLP endpoint, counters and histograms appear in Jaeger/Tempo/Grafana; without telemetry feature, zero overhead
- Failure state exposed: None — recording functions silently no-op on any failure

## Inputs

- `crates/assay-core/src/telemetry.rs` — recording functions from T02
- `crates/assay-core/src/pipeline.rs` — `run_session`, `execute_session` (gate_evaluate scope), `launch_agent` call site
- `crates/assay-core/src/orchestrate/merge_runner.rs` — `merge_completed_sessions` entry

## Expected Output

- `crates/assay-core/src/pipeline.rs` — 4 recording call sites (session, gate counter, gate latency, agent duration)
- `crates/assay-core/src/orchestrate/merge_runner.rs` — 1 recording call site (merges attempted)
