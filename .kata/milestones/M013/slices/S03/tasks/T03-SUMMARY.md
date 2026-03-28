---
id: T03
parent: S03
milestone: M013
provides:
  - 5 metric recording call sites across pipeline.rs and merge_runner.rs
  - Instant-based latency measurement for gate_eval and agent_run histograms
  - Unconditional calls that no-op when metrics not initialized
key_files:
  - crates/assay-core/src/pipeline.rs
  - crates/assay-core/src/orchestrate/merge_runner.rs
key_decisions:
  - "agent_run_duration recorded only on successful agent runs (error paths return early before recording); histogram captures happy-path latency distribution"
patterns_established:
  - "Instrumentation pattern: unconditional crate::telemetry::record_*() calls at site — no cfg guards, no feature checks; recording functions handle no-op internally"
  - "Latency pattern: Instant::now() before scope, .elapsed().as_secs_f64() * 1000.0 after scope to convert to milliseconds"
observability_surfaces:
  - "Five OTel metrics emitted during pipeline execution when telemetry feature enabled: sessions.launched, gates.evaluated, merges.attempted counters; gate_eval_latency, agent_run_duration histograms (ms)"
duration: 8min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T03: Instrument pipeline and merge code paths

**Placed 5 OTel metric recording calls at pipeline and merge instrumentation sites with Instant-based latency measurement for histograms**

## What Happened

Added the five metric recording calls from T02's API surface to their instrumentation sites:

1. `record_session_launched()` at `run_session` entry — counts every pipeline session.
2. `record_gate_evaluated()` + `record_gate_eval_latency_ms()` inside the `gate_evaluate` info_span scope — counts gate evaluations and records latency in ms.
3. `record_agent_run_duration_ms()` wrapping the agent launch stage — records successful agent run duration in ms via `Instant` timing outside the info_span.
4. `record_merge_attempted()` at `merge_completed_sessions` entry after the `Instant::now()` start marker.

All calls are unconditional — no `cfg` guards needed since the recording functions no-op when OnceLock handles are uninitialized.

## Verification

- `just ready` — all 1516 tests pass, zero regressions
- `cargo build -p assay-core` — default build clean (no new deps)
- `cargo build -p assay-core --features telemetry` — telemetry build clean
- `cargo test -p assay-core --test otel_metrics --features telemetry` — 4 contract tests pass
- `grep` confirms 5 call sites across 2 files:
  - `pipeline.rs:987` — `record_session_launched()`
  - `pipeline.rs:847` — `record_gate_evaluated()`
  - `pipeline.rs:887` — `record_gate_eval_latency_ms()`
  - `pipeline.rs:839` — `record_agent_run_duration_ms()`
  - `merge_runner.rs:60` — `record_merge_attempted()`

### Slice-level verification status (T03 is final task):
- [x] `cargo test -p assay-core --test otel_metrics --features telemetry` — 4 pass
- [x] `cargo build -p assay-core` — clean
- [x] `cargo build -p assay-core --features telemetry` — clean
- [x] `just ready` — 1516 tests pass

## Diagnostics

When running with `--features telemetry` and an OTLP endpoint, all five metrics appear in the collector. Without telemetry feature, recording functions are empty stubs — zero overhead. No local inspection surface (metrics are push-only to OTLP collector).

## Deviations

- Function names in T03-PLAN used `record_gate_eval_latency` / `record_agent_run_duration` / `record_merges_attempted` but T02 implemented them as `record_gate_eval_latency_ms` / `record_agent_run_duration_ms` / `record_merge_attempted`. Used the actual T02 names.
- `agent_run_duration` is recorded only after successful agent runs — the `?` operator propagates errors before the recording call. This is intentional: error paths already carry `elapsed` in the PipelineError struct, and the histogram captures happy-path latency distribution.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/pipeline.rs` — Added 4 metric recording calls (session launched, gate evaluated, gate latency, agent duration)
- `crates/assay-core/src/orchestrate/merge_runner.rs` — Added 1 metric recording call (merge attempted)
