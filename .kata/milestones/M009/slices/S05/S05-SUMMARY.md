---
id: S05
parent: M009
milestone: M009
provides:
  - telemetry Cargo feature on assay-core and assay-cli gating all OTel deps
  - init_tracing() conditionally adds OTLP export layer when feature enabled and endpoint configured
  - TracingConfig.otlp_endpoint field (unconditional) read from OTEL_EXPORTER_OTLP_ENDPOINT env var
  - TracingGuard::drop() flushes pending spans via SdkTracerProvider::shutdown()
  - extract_traceparent() + inject_traceparent() helpers for W3C trace context propagation
  - TRACEPARENT env var injected into launch_agent() and launch_agent_streaming() subprocess spawns
  - Graceful degradation: OTel init failure emits tracing::warn!, continues without OTel layer
  - Verified dep isolation: default build has 0 OTel crates; telemetry feature adds 13
requires:
  - slice: S01
    provides: Layered subscriber architecture (init_tracing, TracingConfig, TracingGuard), tracing macros throughout codebase
  - slice: S02
    provides: Pipeline spans (run_session, setup_session, stage spans) consumed by OTLP exporter
  - slice: S03
    provides: Orchestration spans (DAG/Mesh/Gossip root+session spans) consumed by OTLP exporter
affects: []
key_files:
  - Cargo.toml
  - crates/assay-core/Cargo.toml
  - crates/assay-cli/Cargo.toml
  - crates/assay-core/src/telemetry.rs
  - crates/assay-core/src/pipeline.rs
  - crates/assay-core/tests/telemetry_otlp.rs
  - crates/assay-cli/src/main.rs
key_decisions:
  - "D143: rt-tokio reuses existing assay-core tokio dep — no scoped runtime needed (supersedes D127)"
  - "D144: http-proto + hyper-client transport avoids reqwest version conflict with deny.toml"
  - "D146: Option<L> with .with() for conditional OTel layer — None is a no-op, no type gymnastics"
  - "D147: SdkTracerProvider stored in TracingGuard — no global shutdown fn in otel 0.31"
  - "D148: TRACEPARENT captured before thread::spawn — OTel context is thread-local"
patterns_established:
  - "cfg-gated OTel init with graceful degradation: build in closure → match Ok/Err → warn on failure"
  - "Feature flag forwarding: assay-cli/telemetry enables assay-core/telemetry"
  - "cfg-gated inject_traceparent(&mut Command) for adding trace context to any subprocess"
  - "Env-var activation: OTEL_EXPORTER_OTLP_ENDPOINT controls OTLP, zero code changes needed"
observability_surfaces:
  - "tracing::warn!(endpoint, error, \"OTLP exporter init failed; trace export disabled\") — always visible at default log level"
  - "tracing::debug!(\"extract_traceparent: no active span; TRACEPARENT not injected\") — visible at RUST_LOG=debug"
  - "RUST_LOG=opentelemetry=debug reveals BatchSpanProcessor drops and flush behavior"
  - "cargo tree -p assay-cli | grep opentelemetry — empty confirms dep isolation"
drill_down_paths:
  - .kata/milestones/M009/slices/S05/tasks/T01-SUMMARY.md
  - .kata/milestones/M009/slices/S05/tasks/T02-SUMMARY.md
  - .kata/milestones/M009/slices/S05/tasks/T03-SUMMARY.md
  - .kata/milestones/M009/slices/S05/tasks/T04-SUMMARY.md
duration: ~45min
verification_result: passed
completed_at: 2026-03-26T13:00:00Z
---

# S05: OTLP export and trace context propagation

**Feature-gated OTLP export layer with scoped shutdown, W3C TRACEPARENT propagation into subprocess spawns, and verified zero-dep default build**

## What Happened

S05 delivered the final piece of M009: OTLP export and trace context propagation across subprocess boundaries.

**T01** established the foundation: four OTel workspace dependencies (opentelemetry 0.31, opentelemetry_sdk 0.31 with rt-tokio, opentelemetry-otlp 0.31 with http-proto+hyper-client, tracing-opentelemetry 0.32) added as optional crate-level deps behind a `telemetry` feature on assay-core, with a forwarding `telemetry` feature on assay-cli. The `registry` feature was added to the tracing-subscriber workspace dep. Red-state integration tests were created to define contracts for T02 and T03 before any OTel code existed.

**T02** implemented the OTel layer in `telemetry.rs`. `TracingConfig` gained `otlp_endpoint: Option<String>` (always-present, feature-agnostic). Behind `#[cfg(feature = "telemetry")]`, `build_otel_layer()` sets `TraceContextPropagator` as the global propagator, builds an HTTP OTLP `SpanExporter`, creates an `SdkTracerProvider` with batch export, and returns a `tracing_opentelemetry` layer. The `Option<L>` pattern with tracing-subscriber's `.with()` avoids type divergence — `None` is a no-op layer. `TracingGuard` stores the `SdkTracerProvider` and calls `provider.shutdown()` on drop for deterministic span flushing (otel 0.31 has no global shutdown function). OTel init failures produce a `tracing::warn!` and fall back to fmt-only.

**T03** delivered TRACEPARENT injection into both agent subprocess launch paths. `extract_traceparent()` uses `OpenTelemetrySpanExt::context()` and the global propagator to serialize the active span into a W3C traceparent string. `inject_traceparent()` wraps this for `Command::env()` injection. A key pattern: `launch_agent_streaming()` captures the traceparent value *before* `thread::spawn` because OTel context is thread-local. The red-state integration test was rewritten with a real `SdkTracerProvider` (no exporter) and `TraceContextPropagator`, proving TRACEPARENT appears in subprocess output matching `00-{32hex}-{16hex}-{2hex}`.

**T04** wired the CLI endpoint config: `tracing_config_for()` reads `OTEL_EXPORTER_OTLP_ENDPOINT` and applies it to both default and MCP configs. `just ready` was run, confirming full workspace green.

## Verification

| Check | Result |
|-------|--------|
| `cargo tree -p assay-cli \| grep opentelemetry` | empty — 0 OTel crates in default build ✓ |
| `cargo tree -p assay-cli --features telemetry \| grep opentelemetry` | 13 crates ✓ |
| `cargo test -p assay-core --test telemetry_otlp --features telemetry` | 2/2 passed ✓ |
| `cargo build -p assay-cli --features telemetry` | compiles without errors ✓ |
| `cargo deny check bans` | clean ✓ |
| `just ready` | All checks passed (fmt, clippy, test, deny) ✓ |

## Requirements Advanced

- R064 (OTLP trace export) — init_tracing() adds OTLP layer when feature enabled + endpoint set; TracingGuard flushes on drop; graceful degradation on init failure
- R065 (Trace context propagation) — TRACEPARENT injected into both launch_agent() and launch_agent_streaming(); W3C format proven by integration test

## Requirements Validated

- R064 — telemetry feature gates all OTel deps; default build has zero OTel crates (cargo tree verified); feature build compiles; integration tests pass; `just ready` green. Real Jaeger validation is UAT.
- R065 — W3C TRACEPARENT injection proven by integration test asserting valid `00-{32hex}-{16hex}-{2hex}` format in subprocess env; both launch paths covered; thread-local span context handled correctly.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- **otel 0.31 has no global shutdown_tracer_provider()**: Task plan referenced this function. Instead, `SdkTracerProvider::shutdown()` is called directly on the stored provider in `TracingGuard::drop()`. Same effect, different API surface. Captured as D147.
- **Cargo workspace `optional` not supported**: Task plan specified `optional = true` at workspace level. Cargo does not support this — `optional = true` must be on crate-level dep declarations only. Fixed by keeping the workspace entry version-only and marking `optional = true` in assay-core's Cargo.toml.
- **`with_batch_exporter` API change in 0.31**: The builder takes only the exporter; runtime is configured via the `rt-tokio` feature flag on `opentelemetry_sdk`. Plan referenced a two-argument form.

## Known Limitations

- Real Jaeger/Tempo integration is UAT only — no automated test exercises an actual OTLP endpoint.
- `assay-tui` binary does not pass `--features telemetry` through to assay-core — TUI users who want OTLP need to rebuild with the feature flag manually.
- BatchSpanProcessor may drop spans on abnormal termination (SIGKILL, panic) if `TracingGuard` is not dropped cleanly. This is a known OTel limitation.

## Follow-ups

- UAT: Start Jaeger (`docker run -p 4318:4318 -p 16686:16686 jaegertracing/all-in-one`), run `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 cargo run --features telemetry -- run manifest.toml`, verify spans appear in Jaeger UI with correct parent-child nesting.
- Consider adding `telemetry` feature to assay-tui crate if TUI users need OTLP export.
- R066 (TUI trace viewer) and R067 (OTel metrics) remain deferred.

## Files Created/Modified

- `Cargo.toml` — 4 OTel workspace deps + registry feature on tracing-subscriber
- `crates/assay-core/Cargo.toml` — telemetry feature + 4 optional OTel deps
- `crates/assay-cli/Cargo.toml` — telemetry feature forwarding to assay-core/telemetry
- `crates/assay-core/src/telemetry.rs` — otlp_endpoint field, cfg-gated OTel layer, TracingGuard shutdown, graceful degradation
- `crates/assay-core/src/pipeline.rs` — extract_traceparent(), inject_traceparent() helpers; TRACEPARENT injection in both launch paths
- `crates/assay-core/tests/telemetry_otlp.rs` — 2 integration tests (OTel init, TRACEPARENT injection)
- `crates/assay-cli/src/main.rs` — tracing_config_for() reads OTEL_EXPORTER_OTLP_ENDPOINT

## Forward Intelligence

### What the next slice should know
- M009 is now complete — all slices S01–S05 are done.
- To enable OTLP in production: `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 cargo run --features telemetry`.
- The `telemetry` feature is additive — default builds remain zero-dep for OTel.
- Both `TracingConfig.otlp_endpoint` and TRACEPARENT injection are unconditionally compiled structs/fields; only the OTel behavior is cfg-gated. This means the CLI config and struct serde always work.

### What's fragile
- `TracingGuard` must be held for the full process lifetime — if dropped early, pending OTLP spans will not be exported. The guard is returned from `init_tracing()` and should be bound in `main()`.
- Thread-local OTel span context: any new subprocess spawn added to the codebase must capture traceparent before `thread::spawn` if it needs propagation.

### Authoritative diagnostics
- `RUST_LOG=opentelemetry=debug` — shows BatchSpanProcessor queue depth, export attempts, and flush results.
- `cargo tree -p assay-cli | grep opentelemetry` — canonical dep isolation check; must return empty for default build.
- `tracing::warn!` on OTLP init failure — if traces don't appear in Jaeger, check stderr for this log line.

### What assumptions changed
- D127 assumed assay-core had no tokio dep and would need an isolated async runtime for OTLP export. In practice, assay-core already had `tokio` as a direct dependency (added in M009/S01 for tracing non-blocking writer). No scoped runtime was needed.
