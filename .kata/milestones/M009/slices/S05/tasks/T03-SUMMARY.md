---
id: T03
parent: S05
milestone: M009
provides:
  - extract_traceparent() helper — cfg-gated function extracting W3C TRACEPARENT from active OTel span context
  - inject_traceparent() helper — cfg-gated function adding TRACEPARENT env var to std::process::Command
  - launch_agent() TRACEPARENT injection — env var added to claude subprocess when telemetry feature enabled and span active
  - launch_agent_streaming() TRACEPARENT injection — traceparent captured in outer scope before thread spawn, injected into Command
  - Integration test test_traceparent_injected_in_subprocess — proves TRACEPARENT appears in subprocess env with valid W3C format
key_files:
  - crates/assay-core/src/pipeline.rs
  - crates/assay-core/tests/telemetry_otlp.rs
  - crates/assay-core/src/telemetry.rs
key_decisions:
  - "TRACEPARENT extraction factored into two helpers (extract_traceparent + inject_traceparent) to avoid duplicating propagator logic between launch_agent and launch_agent_streaming"
  - "launch_agent_streaming captures traceparent value in outer scope before thread::spawn because OTel span context is thread-local"
patterns_established:
  - "cfg-gated inject_traceparent(&mut Command) pattern for adding trace context to any subprocess"
  - "Test pattern: set up SdkTracerProvider + TraceContextPropagator, use set_default subscriber (not try_init) for isolated test scope"
observability_surfaces:
  - "tracing::debug!(\"extract_traceparent: no active span; TRACEPARENT not injected\") when no OTel span context available"
  - "Child process reads $TRACEPARENT to correlate its traces with parent orchestration"
duration: 12min
verification_result: passed
completed_at: 2026-03-26T12:00:00Z
blocker_discovered: false
---

# T03: Inject TRACEPARENT into launch_agent and launch_agent_streaming subprocess spawns

**Feature-gated W3C TRACEPARENT injection into both agent subprocess launch paths via extracted propagator helpers, with integration test proving valid trace context in child environment**

## What Happened

Added two cfg-gated helper functions to pipeline.rs: `extract_traceparent()` uses `tracing_opentelemetry::OpenTelemetrySpanExt` to get the current span's OTel context and the global text map propagator to serialize it into a TRACEPARENT string. `inject_traceparent()` wraps this to call `.env("TRACEPARENT", value)` on a Command builder.

`launch_agent()` was refactored to build the Command in two steps (construct then spawn) so the cfg-gated injection can be inserted between them. `launch_agent_streaming()` captures the traceparent value in the outer scope before `thread::spawn` because OTel span context is thread-local — the spawned thread wouldn't see the parent's active span.

The red-state integration test was rewritten to set up a real OTel provider (SdkTracerProvider with no exporter), register TraceContextPropagator, wire a tracing-opentelemetry layer via `set_default` subscriber, create a parent span, extract TRACEPARENT via the same propagator logic, inject it into a subprocess, and assert the output matches W3C format `00-{32hex}-{16hex}-{2hex}`.

Also fixed a pre-existing clippy collapsible_if warning in telemetry.rs TracingGuard::drop().

## Verification

- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — 2 passed (test_otel_layer_init_compiles, test_traceparent_injected_in_subprocess)
- `cargo test -p assay-core --lib -- pipeline` — 20 passed, 0 failed (no regressions, default build unaffected)
- `cargo clippy -p assay-core --features telemetry -- -D warnings` — clean
- `cargo tree -p assay-cli | grep opentelemetry` — empty (default build has no OTel deps)
- `cargo build -p assay-cli --features telemetry` — compiles successfully

## Diagnostics

- When no active span: `tracing::debug!("extract_traceparent: no active span; TRACEPARENT not injected")` — visible at `RUST_LOG=debug`
- When propagator not set: `extract_traceparent()` returns None (empty carrier) — no TRACEPARENT injected, debug log explains why
- Child process reads `$TRACEPARENT` to correlate traces; absence means injection was skipped (check debug logs)

## Deviations

- Fixed pre-existing clippy collapsible_if warning in telemetry.rs TracingGuard::drop() — trivial cleanup, not a plan deviation
- Test uses `set_default` subscriber instead of `try_init` to avoid conflicts with other tests that may have already set a global subscriber

## Known Issues

None

## Files Created/Modified

- `crates/assay-core/src/pipeline.rs` — Added extract_traceparent(), inject_traceparent() helpers; injected TRACEPARENT into launch_agent() and launch_agent_streaming()
- `crates/assay-core/tests/telemetry_otlp.rs` — Rewrote test_traceparent_injected_in_subprocess with real OTel provider and propagator
- `crates/assay-core/src/telemetry.rs` — Fixed collapsible_if clippy warning in TracingGuard::drop()
