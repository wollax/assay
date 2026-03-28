---
id: T02
parent: S03
milestone: M013
provides:
  - build_otel_metrics() function returning SdkMeterProvider
  - Five OnceLock global metric handles (counters + histograms)
  - init_metric_handles(&Meter) to populate handles
  - Five pub recording functions (no-op without telemetry feature)
  - TracingGuard with _meter_provider and D179 shutdown ordering
key_files:
  - crates/assay-core/src/telemetry.rs
key_decisions:
  - "Meter name 'assay' used for all metric instruments, consistent with tracer name"
  - "Metric names use dotted notation: assay.sessions.launched, assay.gates.evaluated, etc."
  - "D179: meter shutdown before tracer shutdown in TracingGuard Drop impl"
patterns_established:
  - "OnceLock pattern for global metric handles — check .get() in recording functions for safe no-op"
  - "Dual cfg stubs: telemetry feature has real impl, non-telemetry has empty functions — both pub with same signature"
observability_surfaces:
  - "tracing::debug! on successful metrics provider init"
  - "tracing::warn! on metrics provider init failure (includes endpoint and error)"
  - "eprintln! on meter provider shutdown failure in TracingGuard::drop"
duration: 10min
verification_result: passed
completed_at: 2025-03-28T12:00:00Z
blocker_discovered: false
---

# T02: build_otel_metrics, init_metric_handles, and TracingGuard integration

**OTel metrics infrastructure: meter provider builder, 5 OnceLock global handles, 5 thin recording functions, TracingGuard integration with D179 meter-before-tracer shutdown**

## What Happened

Implemented the full metrics infrastructure in `telemetry.rs`:

1. `build_otel_metrics(config)` mirrors `build_otel_layer` — reads `otlp_endpoint`, builds `MetricExporter` with HTTP transport, constructs `SdkMeterProvider` with periodic exporter. Returns `None` on missing endpoint or failure.

2. Five `OnceLock` globals store metric handles: `SESSIONS_LAUNCHED`, `GATES_EVALUATED`, `MERGES_ATTEMPTED` (counters), `GATE_EVAL_LATENCY`, `AGENT_RUN_DURATION` (histograms). All behind `#[cfg(feature = "telemetry")]`.

3. `init_metric_handles(&Meter)` creates instruments via `meter.u64_counter().build()` / `meter.f64_histogram().build()` and sets each OnceLock. Public for test access.

4. Five thin recording functions (`record_session_launched`, `record_gate_evaluated`, `record_merge_attempted`, `record_gate_eval_latency_ms`, `record_agent_run_duration_ms`) — each has a real `cfg(telemetry)` variant that checks OnceLock and calls `.add(1, &[])` or `.record(value, &[])`, plus an empty `cfg(not(telemetry))` stub. All `pub` without cfg guards on callers.

5. `TracingGuard` gains `_meter_provider: Option<SdkMeterProvider>`. Drop impl shuts down meter FIRST, then tracer (D179). `init_tracing` calls `build_otel_metrics`, initializes handles if provider exists, stores provider in guard.

## Verification

- `cargo test -p assay-core --test otel_metrics --features telemetry` — all 4 contract tests pass
- `cargo build -p assay-core` — default build compiles clean (recording functions are empty stubs)
- `cargo test -p assay-core --lib` — 691 existing tests pass
- Grep confirms `_meter_provider` shutdown before `_tracer_provider` in Drop impl (D179)

## Diagnostics

- Check `TracingGuard` Drop impl for shutdown ordering — meter on line 532, tracer on line 541
- OnceLock globals: `SESSIONS_LAUNCHED`, `GATES_EVALUATED`, `MERGES_ATTEMPTED`, `GATE_EVAL_LATENCY`, `AGENT_RUN_DURATION`
- Init failure: `tracing::warn!` with endpoint and error string
- Shutdown failure: `eprintln!` in Drop with error details

## Deviations

- Function names follow contract tests (source of truth) rather than task plan: `record_merge_attempted` not `record_merges_attempted`, `record_gate_eval_latency_ms` / `record_agent_run_duration_ms` with `_ms` suffix.
- Metric instrument names use dotted convention (`assay.sessions.launched`) rather than being specified in the plan.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/telemetry.rs` — Added `build_otel_metrics`, 5 OnceLock globals, `init_metric_handles`, 5 recording functions (with cfg stubs), updated `TracingGuard` with `_meter_provider` field and D179 Drop ordering, updated `init_tracing` to wire metrics
