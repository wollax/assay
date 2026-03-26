---
id: S05
parent: M009
milestone: M009
provides:
  - telemetry feature flag on assay-core and assay-cli gating all OTel deps
  - OTel OTLP exporter layer in init_tracing() subscriber chain with graceful degradation
  - TracingGuard::drop() deterministic shutdown of SdkTracerProvider
  - W3C TRACEPARENT env var injection into launch_agent() and launch_agent_streaming() subprocesses
  - CLI reads OTEL_EXPORTER_OTLP_ENDPOINT env var to activate OTLP export
  - Default cargo build has zero OTel/tokio dep contamination
requires:
  - slice: S01
    provides: tracing-subscriber layered initialization (init_tracing, TracingConfig, TracingGuard)
  - slice: S02
    provides: "#[instrument] spans on pipeline stage functions"
  - slice: S03
    provides: Orchestration root/session/merge spans with cross-thread parenting
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
  - "D143: Use rt-tokio with existing runtime, no scoped runtime — supersedes D127"
  - "D144: http-proto + hyper-client transport for opentelemetry-otlp — avoids reqwest version conflict"
  - "D145: Test-first contract + dep isolation assertions; real Jaeger is UAT only"
  - "OTel layer uses Option<L> pattern with tracing-subscriber .with(Option<L>) for zero type-gymnastics"
  - "SdkTracerProvider stored in TracingGuard for shutdown on drop (otel 0.31 has no global shutdown)"
  - "TRACEPARENT captured in outer scope before thread::spawn (OTel context is thread-local)"
patterns_established:
  - "Feature flag forwarding: assay-cli/telemetry enables assay-core/telemetry"
  - "cfg-gated OTel init with graceful degradation: build in closure → match Ok/Err → warn on failure"
  - "cfg-gated inject_traceparent(&mut Command) for adding trace context to any subprocess"
  - "Env-var-driven OTel activation: set OTEL_EXPORTER_OTLP_ENDPOINT to enable, unset to disable"
observability_surfaces:
  - "tracing::warn!(endpoint, error) on OTLP init failure — always visible at default log level"
  - "tracing::debug!(\"extract_traceparent: no active span\") when no OTel span context available"
  - "RUST_LOG=opentelemetry=debug shows BatchSpanProcessor behavior"
  - "cargo tree -p assay-cli | grep opentelemetry — must return empty for default build"
drill_down_paths:
  - .kata/milestones/M009/slices/S05/tasks/T01-SUMMARY.md
  - .kata/milestones/M009/slices/S05/tasks/T02-SUMMARY.md
  - .kata/milestones/M009/slices/S05/tasks/T03-SUMMARY.md
  - .kata/milestones/M009/slices/S05/tasks/T04-SUMMARY.md
duration: 45min
verification_result: passed
completed_at: 2026-03-26T12:30:00Z
---

# S05: OTLP export and trace context propagation

**Feature-flagged OTLP exporter with http-proto transport, W3C TRACEPARENT subprocess injection, CLI env-var activation, and verified zero-OTel default build isolation**

## What Happened

T01 established the dependency foundation: four OTel workspace deps (opentelemetry 0.31, opentelemetry_sdk 0.31, opentelemetry-otlp 0.31, tracing-opentelemetry 0.32) as optional deps behind a `telemetry` feature on assay-core, forwarded by assay-cli. Red-state integration tests defined the OTLP init and TRACEPARENT injection contracts before any OTel code existed. Used http-proto + hyper-client transport to avoid reqwest version conflicts with the workspace's jsonschema dependency.

T02 implemented the OTel OTLP layer inside `init_tracing()`. Added `otlp_endpoint: Option<String>` to TracingConfig (unconditional field). Behind `#[cfg(feature = "telemetry")]`, `build_otel_layer()` sets the TraceContextPropagator, creates an HTTP OTLP SpanExporter, builds an SdkTracerProvider with batch export, and returns a tracing-opentelemetry layer. Uses the `Option<Layer>` pattern — `None` is a no-op layer when OTel is off or fails. TracingGuard stores the SdkTracerProvider and calls `shutdown()` on drop for deterministic span flushing. OTel init failures emit `tracing::warn!` and continue without the OTel layer.

T03 added TRACEPARENT injection to both subprocess launch paths. Two cfg-gated helpers (`extract_traceparent()` and `inject_traceparent()`) use the global text map propagator to serialize the current span's OTel context into a W3C TRACEPARENT header and inject it via `Command::env()`. In `launch_agent_streaming()`, the traceparent value is captured in the outer scope before `thread::spawn` because OTel span context is thread-local.

T04 closed the integration loop: CLI's `tracing_config_for()` reads `OTEL_EXPORTER_OTLP_ENDPOINT` and populates `otlp_endpoint` for both default and MCP configs. Dep isolation verified: default build has 0 OTel deps, telemetry feature adds 13. `just ready` passes clean.

## Verification

- `cargo tree -p assay-cli | grep opentelemetry` — empty (default build has no OTel deps) ✓
- `cargo tree -p assay-cli --features telemetry | grep opentelemetry` — shows 13 OTel crates ✓
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — 2/2 passed (OTel init + TRACEPARENT injection) ✓
- `cargo build -p assay-cli --features telemetry` — compiles successfully ✓
- `cargo deny check bans` — no new ban violations ✓
- `just ready` — all checks passed (fmt, clippy, all tests, deny) ✓

## Requirements Advanced

- R064 — OTLP trace export fully implemented: feature-flagged exporter with graceful degradation, endpoint via env var
- R065 — Trace context propagation fully implemented: TRACEPARENT injection in both launch_agent and launch_agent_streaming

## Requirements Validated

- R064 — Feature-flagged OTLP exporter compiles and initializes with configured endpoint; default build has zero OTel deps; graceful degradation on init failure. Real Jaeger validation is UAT.
- R065 — Integration test proves TRACEPARENT appears in subprocess env with valid W3C format `00-{32hex}-{16hex}-{2hex}` when telemetry feature enabled and span active.

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- D127 superseded by D143: original plan called for a scoped `new_current_thread` tokio runtime. In practice, assay-core already has tokio as a direct dep, so `rt-tokio` reuses the existing runtime with zero overhead.
- OTel 0.31 API differs from plan: `shutdown_tracer_provider()` global function doesn't exist — shutdown is on SdkTracerProvider directly. `with_batch_exporter` takes only the exporter (runtime configured via feature flag).
- `optional = true` cannot be set at workspace level in Cargo.toml — only on crate-level deps. Fixed by keeping `optional = true` in assay-core's Cargo.toml only.

## Known Limitations

- Real OTLP endpoint testing (Jaeger/Grafana Tempo) is UAT only — no integration test spins up a collector
- OTel init failure in TracingGuard::drop() uses `eprintln!` (can't use tracing inside drop of the tracing infrastructure)
- TRACEPARENT injection requires both the `telemetry` feature AND an active OTel span — without both, no injection occurs (debug log explains why)

## Follow-ups

- None — S05 is the final slice of M009. All milestone success criteria are addressed.

## Files Created/Modified

- `Cargo.toml` — Added 4 OTel workspace deps + registry feature on tracing-subscriber
- `crates/assay-core/Cargo.toml` — Added telemetry feature + 4 optional OTel deps
- `crates/assay-cli/Cargo.toml` — Added telemetry feature forwarding to assay-core/telemetry
- `crates/assay-core/src/telemetry.rs` — OTel OTLP layer, otlp_endpoint field, TracingGuard shutdown, graceful degradation
- `crates/assay-core/src/pipeline.rs` — extract_traceparent(), inject_traceparent() helpers, TRACEPARENT injection in both launch functions
- `crates/assay-core/tests/telemetry_otlp.rs` — 2 integration tests (OTel init + TRACEPARENT injection)
- `crates/assay-cli/src/main.rs` — tracing_config_for() reads OTEL_EXPORTER_OTLP_ENDPOINT

## Forward Intelligence

### What the next slice should know
- S05 is the final slice of M009. No downstream slices exist. The milestone is complete after this summary.

### What's fragile
- OTel 0.31 API surface is relatively new and may change in future releases — the `build_otel_layer()` function in telemetry.rs is the single point of OTel API coupling
- hyper-client transport avoids reqwest conflict but locks the OTel transport to hyper — changing to reqwest or tonic would require resolving the version conflict

### Authoritative diagnostics
- `cargo tree -p assay-cli | grep opentelemetry` — the definitive dep isolation check; if this returns non-empty, the feature flag leaked
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — the two contract tests prove OTel init and TRACEPARENT injection work

### What assumptions changed
- D127 assumed assay-core had no tokio dep — it already did, making the scoped runtime unnecessary (D143)
- OTel 0.31 API differs from documentation examples that reference older versions — several function signatures changed
