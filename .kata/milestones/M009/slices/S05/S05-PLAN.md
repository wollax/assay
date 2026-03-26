# S05: OTLP export and trace context propagation

**Goal:** Feature-flagged OTLP exporter sends spans to a collector (Jaeger/Tempo). TRACEPARENT env var injected into subprocess spawns. Default `cargo build` has zero OTel/new deps. `just ready` green.
**Demo:** `cargo build --features telemetry` compiles with OTel support. `cargo tree -p assay-cli | grep opentelemetry` returns empty (default). TRACEPARENT injection verified by integration test. OTel init failure produces a visible warning, not a silent fallback.

## Must-Haves

- `telemetry` feature on assay-core enables optional OTel deps (opentelemetry, opentelemetry_sdk, opentelemetry-otlp, tracing-opentelemetry)
- `telemetry` feature on assay-cli enables `assay-core/telemetry`
- `init_tracing()` conditionally adds OTel tracing layer when feature enabled and `otlp_endpoint` is Some
- `TracingGuard::drop()` calls `opentelemetry::global::shutdown_tracer_provider()` behind `#[cfg(feature = "telemetry")]`
- `TracingConfig` gains `otlp_endpoint: Option<String>` (always-present field, only acted on when feature compiled in)
- `TRACEPARENT` env var injected into `launch_agent()` and `launch_agent_streaming()` subprocess `Command::env()` calls when feature enabled and an active span exists
- `opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new())` called before subscriber init
- `cargo build -p assay-cli` (no features) produces no opentelemetry crates in dep tree
- `cargo build -p assay-cli --features telemetry` compiles successfully
- OTel init failure emits `tracing::warn!` with endpoint and error, then continues without OTel layer (graceful degradation)
- `just ready` passes with all new tests green
- `tracing-subscriber` workspace features include `registry` (required by tracing-opentelemetry)

## Proof Level

- This slice proves: contract + integration (feature-flag dep isolation, TRACEPARENT injection via subprocess test, OTel layer compilation)
- Real runtime required: no (real Jaeger is UAT only)
- Human/UAT required: yes — Jaeger UI trace visualization is UAT

## Verification

- `cargo tree -p assay-cli | grep opentelemetry` — must return empty (default build has no OTel deps)
- `cargo tree -p assay-cli --features telemetry | grep opentelemetry` — must show OTel crate tree
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — OTLP layer init + TRACEPARENT injection tests pass
- `cargo build -p assay-cli --features telemetry` — compiles without errors
- `cargo deny check bans` — no new ban violations
- `just ready` — full workspace green

## Observability / Diagnostics

- Runtime signals: `tracing::warn!(endpoint, error, "OTLP exporter init failed; trace export disabled")` on OTel setup failure; `tracing::debug!("launch_agent: no active span; TRACEPARENT not injected")` when no span context available
- Inspection surfaces: `RUST_LOG=opentelemetry=debug` reveals BatchSpanProcessor drops; `OTEL_EXPORTER_OTLP_ENDPOINT` env var controls endpoint
- Failure visibility: OTel init failure is always visible via warn log (never silent); TRACEPARENT absence is visible at debug level
- Redaction constraints: none (no secrets in trace context)

## Integration Closure

- Upstream surfaces consumed: `assay_core::telemetry` (init_tracing, TracingConfig, TracingGuard from S01); `assay_core::pipeline` (launch_agent, launch_agent_streaming from S02); tracing-subscriber registry layer architecture
- New wiring introduced in this slice: OTel layer added to subscriber chain in init_tracing(); TRACEPARENT injected into subprocess Command::env(); CLI reads OTEL_EXPORTER_OTLP_ENDPOINT and populates TracingConfig::otlp_endpoint
- What remains before the milestone is truly usable end-to-end: nothing — S05 is the final slice; after this, all M009 success criteria are met

## Tasks

- [x] **T01: Add OTel workspace deps, feature flags, and red-state integration tests** `est:25m`
  - Why: Establishes the dependency and feature-flag foundation; creates failing tests that define the contract before any OTel code is written
  - Files: `Cargo.toml`, `crates/assay-core/Cargo.toml`, `crates/assay-cli/Cargo.toml`, `crates/assay-core/tests/telemetry_otlp.rs`
  - Do: Add opentelemetry/opentelemetry_sdk/opentelemetry-otlp/tracing-opentelemetry as optional workspace deps. Add `registry` to tracing-subscriber features. Add `telemetry` feature to assay-core and assay-cli. Create red-state integration tests asserting: (1) OTel layer init compiles and returns TracingGuard, (2) TRACEPARENT env var appears in subprocess output, (3) default build dep tree has no opentelemetry. Respect deny.toml — use `http-proto` + `hyper-client` on opentelemetry-otlp to avoid reqwest conflict. Run `cargo deny check bans` to verify no new violations.
  - Verify: `cargo test -p assay-core --test telemetry_otlp --features telemetry` compiles (tests may fail — red state); `cargo deny check bans` passes; `cargo build -p assay-cli` (default) still compiles
  - Done when: Feature flags defined on both crates, OTel deps are optional in workspace, test file exists and compiles with `--features telemetry`, `cargo deny check bans` clean

- [ ] **T02: Implement OTel tracing layer in init_tracing() with feature-flagged TracingGuard shutdown** `est:30m`
  - Why: The core OTel integration — adds the OTLP exporter layer to the subscriber chain and ensures spans are flushed on process exit
  - Files: `crates/assay-core/src/telemetry.rs`
  - Do: Add `otlp_endpoint: Option<String>` to TracingConfig. Behind `#[cfg(feature = "telemetry")]`: set global text map propagator (TraceContextPropagator), build OTel pipeline with `opentelemetry_otlp` (http-proto + hyper-client transport, endpoint from config, rt-tokio runtime), create tracing-opentelemetry layer, add to registry().with() chain. On OTel init failure: emit tracing::warn! and continue without OTel layer. Add `shutdown_tracer_provider()` to TracingGuard::drop() behind cfg. Add `registry` feature to tracing-subscriber usage.
  - Verify: `cargo test -p assay-core --test telemetry_otlp --features telemetry` — OTel init test passes; `cargo build -p assay-cli --features telemetry` compiles
  - Done when: init_tracing() with otlp_endpoint=Some creates an OTel-enabled subscriber; TracingGuard drop flushes OTel; graceful degradation on bad endpoint

- [ ] **T03: Inject TRACEPARENT into launch_agent and launch_agent_streaming subprocess spawns** `est:20m`
  - Why: Delivers R065 (trace context propagation) — child processes receive W3C TRACEPARENT so their traces correlate with the parent orchestration
  - Files: `crates/assay-core/src/pipeline.rs`, `crates/assay-core/tests/telemetry_otlp.rs`
  - Do: Behind `#[cfg(feature = "telemetry")]` in both launch_agent() and launch_agent_streaming(): check Span::current().is_disabled(), if active span exists use global propagator to inject TRACEPARENT into a HashMap, add as Command::env("TRACEPARENT", value). When no active span, emit tracing::debug!. Write integration test: set up OTel subscriber with propagator, create a parent span, call a subprocess (e.g. `env` or `printenv TRACEPARENT`) inside the span, assert TRACEPARENT appears in output matching W3C format `00-{trace_id}-{span_id}-{flags}`.
  - Verify: `cargo test -p assay-core --test telemetry_otlp --features telemetry` — TRACEPARENT test passes
  - Done when: Both launch functions inject TRACEPARENT when feature enabled and span active; integration test proves the value appears in subprocess env

- [ ] **T04: Wire CLI endpoint config, verify feature-flag dep isolation, and run just ready** `est:15m`
  - Why: Closes the integration loop — CLI reads the env var and populates TracingConfig; dep isolation verified by cargo tree; full workspace green
  - Files: `crates/assay-cli/src/main.rs`, `crates/assay-core/tests/telemetry_otlp.rs`
  - Do: In tracing_config_for(), read `OTEL_EXPORTER_OTLP_ENDPOINT` env var and set TracingConfig::otlp_endpoint when present. Add dep-isolation test: `cargo tree -p assay-cli | grep opentelemetry` must return empty. Add feature-enabled dep test: `cargo tree -p assay-cli --features telemetry | grep opentelemetry` must return non-empty. Run `just ready` to verify full workspace green.
  - Verify: `just ready` passes; `cargo tree -p assay-cli | grep opentelemetry` returns empty; `cargo build -p assay-cli --features telemetry` compiles
  - Done when: CLI populates otlp_endpoint from env var; default build has zero OTel deps (verified by test); `just ready` fully green

## Files Likely Touched

- `Cargo.toml` — workspace deps (opentelemetry, opentelemetry_sdk, opentelemetry-otlp, tracing-opentelemetry, opentelemetry-http); tracing-subscriber registry feature
- `crates/assay-core/Cargo.toml` — telemetry feature + optional OTel deps
- `crates/assay-cli/Cargo.toml` — telemetry feature enabling assay-core/telemetry
- `crates/assay-core/src/telemetry.rs` — OTel layer, TracingConfig.otlp_endpoint, TracingGuard shutdown
- `crates/assay-core/src/pipeline.rs` — TRACEPARENT injection in launch_agent + launch_agent_streaming
- `crates/assay-core/tests/telemetry_otlp.rs` — integration tests (OTel init, TRACEPARENT, dep isolation)
- `crates/assay-cli/src/main.rs` — tracing_config_for() reads OTEL_EXPORTER_OTLP_ENDPOINT
- `deny.toml` — potential skip entries if prost/hyper version conflicts arise
