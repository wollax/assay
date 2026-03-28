---
id: S03
milestone: M013
status: ready
---

# S03: OTel Metrics — Context

## Goal

Add OTel counters and histograms to the existing telemetry infrastructure so that `gate run --features telemetry` increments session/gate/merge counters and records per-criterion latency histograms, with clean `MeterProvider` shutdown via `TracingGuard`.

## Why this Slice

The tracing layer (spans + OTLP export) shipped in M009. Metrics add the aggregate-trend layer — counters and histograms that tracing alone can't provide. S03 is independent of S01/S02/S04 and builds directly on the existing `TracingGuard` and OTLP transport. Doing it now closes the R067 gap while the telemetry module is fresh context.

## Scope

### In Scope

- `init_metrics(config: &TracingConfig) -> Option<SdkMeterProvider>` behind `#[cfg(feature = "telemetry")]`
- `TracingGuard.meter_provider: Option<SdkMeterProvider>` — drop triggers shutdown (metrics flushed before traces, per D179)
- Global counters via `OnceLock`: `sessions_launched`, `gates_evaluated`, `merges_attempted`
- Global histograms via `OnceLock`: `gate_eval_latency_ms` (per-criterion granularity), `agent_run_duration_ms`
- Reuse the same `TracingConfig.otlp_endpoint` for metrics export (same collector endpoint as traces)
- Instrument call sites: `run_session`/`run_manifest` for session counter, `evaluate`/`evaluate_all` for gate counter + per-criterion latency, merge paths for merge counter, `launch_agent`/`launch_agent_streaming` for agent run duration
- All behind `#[cfg(feature = "telemetry")]` — default build has zero OTel deps added
- Contract tests proving counter increments and histogram recording
- `just ready` green throughout

### Out of Scope

- Separate metrics endpoint (reuse `otlp_endpoint`)
- CLI summary output of metrics (OTLP export only; CLI output unchanged)
- Metrics dashboards, alerting, or Grafana configuration
- Real OTLP collector validation (UAT only — Jaeger/Tempo receiving metrics is human-verified)
- Any changes to the existing tracing span instrumentation
- Metrics for TUI operations

## Constraints

- D144: http-proto + hyper-client transport for OTLP (metrics uses same transport as traces — avoid reqwest version conflict)
- D147 analogy / D179: `MeterProvider` stored in `TracingGuard` for deterministic shutdown; metrics flush before traces on drop
- Feature gate: all metrics code behind `#[cfg(feature = "telemetry")]`
- D001: zero-trait convention — global `OnceLock<Counter<u64>>` etc., no metric registry trait
- `opentelemetry_sdk` 0.31 API — verify `MeterProvider`, `Counter`, and `Histogram` creation against actual SDK before implementation
- `just ready` must stay green throughout

## Integration Points

### Consumes

- `crates/assay-core/src/telemetry.rs` — `init_tracing()`, `TracingConfig`, `TracingGuard` (metrics layer attaches here)
- `crates/assay-core/src/pipeline.rs` — `run_session()`, `run_manifest()`, `launch_agent()`, `launch_agent_streaming()` (instrument with counters/histograms)
- `crates/assay-core/src/gate/mod.rs` — `evaluate()`, `evaluate_all()` (instrument with gate counter + per-criterion latency)
- `crates/assay-core/src/orchestrate/` — merge paths (instrument with merge counter)
- D144 http-proto + hyper-client transport (reused for metrics OTLP export)

### Produces

- `init_metrics(config: &TracingConfig) -> Option<SdkMeterProvider>` in `assay_core::telemetry`
- `TracingGuard.meter_provider: Option<SdkMeterProvider>` — clean shutdown on drop
- Global counters: `sessions_launched`, `gates_evaluated`, `merges_attempted` (all `Counter<u64>`)
- Global histograms: `gate_eval_latency_ms` (per-criterion), `agent_run_duration_ms` (both `Histogram<f64>`)

## Open Questions

- **OTel 0.31 metrics API surface** — Need to verify `MeterProvider`, `Counter`, and `Histogram` creation against the actual `opentelemetry_sdk` 0.31 crate before designing `init_metrics()`. The API changed significantly from 0.30. This should be retired in research phase by building against the real SDK.
