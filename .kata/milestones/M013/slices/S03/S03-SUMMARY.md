---
id: S03
parent: M013
milestone: M013
provides:
  - build_otel_metrics(config) -> Option<SdkMeterProvider> behind #[cfg(feature = "telemetry")]
  - Five OnceLock global metric handles (SESSIONS_LAUNCHED, GATES_EVALUATED, MERGES_ATTEMPTED counters; GATE_EVAL_LATENCY, AGENT_RUN_DURATION histograms)
  - init_metric_handles(&Meter) to populate handles
  - Five pub recording functions (no-op stubs in default build, real impl under telemetry)
  - TracingGuard._meter_provider field with D179 meter-before-tracer shutdown ordering
  - Five instrumentation call sites in pipeline.rs and merge_runner.rs
  - 4 contract tests in crates/assay-core/tests/otel_metrics.rs
requires: []
affects: []
key_files:
  - Cargo.toml (workspace root — metrics feature added to opentelemetry, opentelemetry_sdk, opentelemetry-otlp)
  - crates/assay-core/src/telemetry.rs (all metrics infrastructure)
  - crates/assay-core/src/pipeline.rs (4 recording call sites)
  - crates/assay-core/src/orchestrate/merge_runner.rs (1 recording call site)
  - crates/assay-core/tests/otel_metrics.rs (4 contract tests)
key_decisions:
  - "D179: meter provider shutdown before tracer provider in TracingGuard::drop"
  - "Dual cfg stubs: telemetry feature has real impl, non-telemetry has empty functions — both pub with same signature; callers need no cfg guards"
  - "OnceLock pattern for global metric handles — recording functions call .get() for safe no-op when uninitialized"
  - "Metric names use dotted notation: assay.sessions.launched, assay.gates.evaluated, assay.merges.attempted, assay.gate_eval.latency_ms, assay.agent_run.duration_ms"
  - "agent_run_duration recorded only on successful agent runs — error paths return early before recording call"
patterns_established:
  - "Instrumentation pattern: unconditional crate::telemetry::record_*() calls at site — no cfg guards; recording functions handle no-op internally"
  - "Latency pattern: Instant::now() before scope, .elapsed().as_secs_f64() * 1000.0 after scope for millisecond histograms"
  - "Contract test pattern: write tests against the intended API surface before implementing it (red state validates the contract)"
observability_surfaces:
  - "tracing::debug! on successful metrics provider init"
  - "tracing::warn! on metrics provider init failure (includes endpoint and error)"
  - "eprintln! on meter provider shutdown failure in TracingGuard::drop (mirrors existing tracer pattern)"
  - "Five OTel metrics emitted to OTLP collector when telemetry feature enabled and OTEL_EXPORTER_OTLP_ENDPOINT configured"
drill_down_paths:
  - .kata/milestones/M013/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M013/slices/S03/tasks/T02-SUMMARY.md
  - .kata/milestones/M013/slices/S03/tasks/T03-SUMMARY.md
duration: 26min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
---

# S03: OTel Metrics

**OTel counters and histograms alongside existing tracing spans, feature-gated under `telemetry`, with clean `MeterProvider` shutdown via `TracingGuard`**

## What Happened

**T01** established the foundation: added the `metrics` feature to the three OTel workspace dependencies (`opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`) in root `Cargo.toml` and wrote 4 red-state contract tests in `otel_metrics.rs` defining the exact public API surface T02 would implement. The tests referenced `record_session_launched`, `init_metric_handles`, etc. that didn't exist yet — intentional red state that validated the contract before any implementation.

**T02** delivered the full metrics infrastructure in `telemetry.rs`: `build_otel_metrics(config)` mirrors `build_otel_layer` using HTTP transport and periodic exporter; five `OnceLock` globals hold the metric handles; `init_metric_handles(&Meter)` populates them via `meter.u64_counter().build()` and `meter.f64_histogram().build()`; five thin recording functions have real `cfg(feature = "telemetry")` variants plus empty `cfg(not(feature = "telemetry"))` stubs — callers need zero `cfg` guards. `TracingGuard` gained `_meter_provider: Option<SdkMeterProvider>` with D179-compliant Drop ordering (meter shutdown fires before tracer shutdown). `init_tracing` was updated to call `build_otel_metrics` and `init_metric_handles` when the provider is successfully constructed.

**T03** placed the five instrumentation call sites: `record_session_launched()` at `run_session` entry; `record_gate_evaluated()` + `record_gate_eval_latency_ms()` inside the `gate_evaluate` info_span using `Instant` timing; `record_agent_run_duration_ms()` wrapping the agent launch stage (happy-path only, before any early-return error path); `record_merge_attempted()` at `merge_completed_sessions` entry. All calls unconditional — recording functions no-op when OnceLock handles are uninitialized.

## Verification

- `cargo test -p assay-core --test otel_metrics --features telemetry` — all 4 contract tests pass
- `cargo build -p assay-core` — default build clean with zero new OTel deps
- `cargo build -p assay-core --features telemetry` — telemetry build clean with metrics support
- `just ready` — all 1516+ tests pass, zero regressions

## Requirements Advanced

- R067 — All OTel metrics infrastructure shipped: counters, histograms, feature isolation, clean shutdown

## Requirements Validated

- R067 — Contract tests prove meter provider construction, counter increment, histogram recording, and no-op behaviour without init. Feature isolation confirmed: `cargo build -p assay-core` has zero new OTel deps; telemetry build compiles with full metrics support. `just ready` green with all existing tests passing.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- Function names in the T03 task plan (`record_gate_eval_latency`, `record_agent_run_duration`, `record_merges_attempted`) differed from T02's actual implementation (`record_gate_eval_latency_ms`, `record_agent_run_duration_ms`, `record_merge_attempted`). T02 names (contract source of truth) were used throughout.
- `agent_run_duration` is recorded only after successful agent runs — the `?` operator propagates errors before the recording call. Intentional: histogram captures happy-path latency distribution; error paths already carry elapsed time in `PipelineError`.

## Known Limitations

- Real OTLP collector verification (Jaeger/Tempo receiving actual metrics) is UAT-only — not validated by contract tests.
- No local inspection surface for metrics (push-only to OTLP collector) — unlike traces there is no `assay metrics list` command.
- Metric attributes are empty (`&[]`) — no labels like spec_slug or session_id on emitted metrics. Adding labels is future work.

## Follow-ups

- UAT: start Jaeger/Tempo, run `gate run` with `--features telemetry` and `OTEL_EXPORTER_OTLP_ENDPOINT` set, confirm counters and histograms appear in the collector UI.
- Future: add attribute labels (e.g. spec_slug on gate_eval_latency) when per-spec dashboarding becomes valuable.

## Files Created/Modified

- `Cargo.toml` — Added `metrics` feature to `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` workspace deps
- `crates/assay-core/src/telemetry.rs` — `build_otel_metrics`, 5 OnceLock globals, `init_metric_handles`, 5 recording functions (with cfg stubs), `TracingGuard._meter_provider`, D179 Drop ordering, `init_tracing` wiring
- `crates/assay-core/src/pipeline.rs` — 4 metric recording call sites
- `crates/assay-core/src/orchestrate/merge_runner.rs` — 1 metric recording call site
- `crates/assay-core/tests/otel_metrics.rs` — 4 contract tests

## Forward Intelligence

### What the next slice should know
- S04 (wizard runnable criteria) is fully independent — it touches `wizard.rs`, `spec.rs`, and `create_spec_from_params` and has no interaction with the metrics infrastructure built here.
- The `record_*` functions are all public and callable from any crate without cfg guards.

### What's fragile
- `init_metric_handles` uses `OnceLock::set()` — if called twice (e.g. in tests that init_tracing more than once), the second call silently fails and the handles remain from the first call. Tests are structured to avoid this.

### Authoritative diagnostics
- `crates/assay-core/src/telemetry.rs` lines around `TracingGuard` Drop impl — verify meter shutdown appears before tracer shutdown (D179).
- `cargo tree -p assay-core --features telemetry -e features | grep metrics` — confirms `metrics` feature is active on SDK deps.

### What assumptions changed
- Original plan described `MetricExporter::builder().with_http()` — actual OTel SDK 0.31 API uses `opentelemetry_otlp::MetricExporter::builder()` with `with_http_exporter()` transport builder. Same semantics, slightly different call chain.
