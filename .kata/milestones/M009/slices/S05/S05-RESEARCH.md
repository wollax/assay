# S05: OTLP export and trace context propagation — Research

**Researched:** 2026-03-25
**Domain:** OpenTelemetry OTLP export, W3C Trace Context propagation, feature-flagged Cargo deps
**Confidence:** HIGH

## Summary

S05 delivers the production-grade observability path: OTLP export to Jaeger/Grafana Tempo behind a `telemetry` feature flag, and W3C TRACEPARENT env var injection into subprocess spawns so agent traces correlate with the parent orchestration trace.

The key finding is that tokio is **already a direct dependency of assay-core** (direct unconditional `tokio.workspace = true` in assay-core/Cargo.toml, present since M009/S01). D127 anticipated needing a scoped `new_current_thread` runtime, but that is unnecessary — `opentelemetry_sdk` with `rt-tokio` reuses the existing runtime. D143 supersedes D127. The OTel SDK deps (opentelemetry, opentelemetry_sdk, tracing-opentelemetry) are large and must be feature-gated to avoid bloating the default build. The TRACEPARENT injection is a targeted change in `launch_agent` and `launch_agent_streaming`.

**Primary recommendation:** Add a `telemetry` feature on assay-core and assay-cli. Use `opentelemetry-otlp` with `default-features = false, features = ["http-proto", "hyper-client"]` to avoid the reqwest version conflict (workspace uses reqwest 0.13 from jsonschema, a workspace dep; opentelemetry-http ships reqwest 0.12 — conflict with `deny.toml multiple-versions = "deny"`). Use `opentelemetry_sdk = { features = ["rt-tokio"] }` and `tracing-opentelemetry = "0.32"`. Add `opentelemetry_sdk::propagation::TraceContextPropagator` for TRACEPARENT extraction. All OTel deps are `optional` and activated only under `#[cfg(feature = "telemetry")]` blocks.

## Recommendation

**Feature flag approach: two-layer opt-in**

1. `assay-core/Cargo.toml` adds `telemetry` feature that enables optional deps: `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `tracing-opentelemetry`.
2. `assay-cli/Cargo.toml` adds `telemetry` feature that enables `assay-core/telemetry`.
3. `#[cfg(feature = "telemetry")]` guards all OTel code in `telemetry.rs`, `pipeline.rs`, and `launch_agent` / `launch_agent_streaming`.
4. `TracingConfig` gains `otlp_endpoint: Option<String>` (always-present field, always-None by default; only acted on when `telemetry` feature is compiled in).
5. `TRACEPARENT` injection in `launch_agent` / `launch_agent_streaming`: extract current span context via `tracing::Span::current()` + `opentelemetry::global::get_text_map_propagator` + inject into `Command::env("TRACEPARENT", ...)`. When feature is disabled, `std::process::Command` inherits the parent environment automatically — no explicit passthrough needed.

**Transport:** `http-proto + hyper-client` (not grpc-tonic, not reqwest-client). This avoids the reqwest version conflict and avoids pulling TLS crates into the dep tree. The OTLP endpoint defaults to `http://localhost:4318` (standard Jaeger/Tempo HTTP port).

**Tokio runtime:** D127 is superseded by D143. Use `opentelemetry_sdk = { features = ["rt-tokio"] }`. The SDK's `BatchSpanProcessor` dispatches async internally — no caller code needs to be async. The CLI's existing `#[tokio::main]` multi-thread runtime is reused with zero overhead.

**OTel init error handling:** `init_tracing()` must surface OTel initialization failures visibly. The recommended shape is: return `TracingGuard` on success (unchanged API for non-telemetry path); when OTel setup fails, emit a `tracing::warn!` with the endpoint and error, then continue without the OTel layer (graceful degradation). Silent fallback without any message is not acceptable.

**Span flush on exit:** Add `opentelemetry::global::shutdown_tracer_provider()` to `TracingGuard::drop()` behind `#[cfg(feature = "telemetry")]`. `Drop` cannot return `Result`, so emit a `tracing::warn!` if the SDK exposes a failure signal. Callers that want explicit control can call a `TracingGuard::shutdown()` method before drop.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Bridging tracing spans to OTel SDK | `tracing-opentelemetry = "0.32"` layer | Direct integration — one `.with(otel_layer)` in `init_tracing()`; maintains parent-child span relationships |
| W3C TRACEPARENT serialization | `opentelemetry_sdk::propagation::TraceContextPropagator` | Standard compliant; handles the `00-traceid-spanid-flags` format correctly |
| OTLP HTTP/protobuf export | `opentelemetry-otlp` `http-proto` + `hyper-client` features | Avoids reqwest version conflict; no TLS overhead; verified clean with `cargo tree \| grep reqwest` (empty) |
| Async OTLP flush | `opentelemetry_sdk` `rt-tokio` feature | Reuses existing tokio dep; `shutdown_tracer_provider()` flushes in-flight spans on process exit |

## Existing Code and Patterns

- `crates/assay-core/src/telemetry.rs` — Contains `init_tracing(TracingConfig) -> TracingGuard`. The `registry().with(filter).with(fmt_layer).with(json_layer)` chain is the extension point: add `.with(otel_layer)` here. `TracingConfig` already has `traces_dir: Option<PathBuf>` — add `otlp_endpoint: Option<String>` with the same pattern. **Note: `traces_dir` and `JsonFileLayer` exist on the `kata/root/M009/S04` branch but are not yet on main; S05 is based on S04's work.**
- `crates/assay-core/src/pipeline.rs` — `launch_agent()` function signature at line 222; `Command::new("claude")` at line 229 — this is the injection point for `.env("TRACEPARENT", traceparent)`. `launch_agent_streaming()` function signature at line 337.
- `crates/assay-cli/src/main.rs` — `tracing_config_for()` returns `TracingConfig` per subcommand. S05 adds logic to check `OTEL_EXPORTER_OTLP_ENDPOINT` env var and populate `TracingConfig::otlp_endpoint` when set.
- `Cargo.toml` workspace — `tokio = { version = "1", features = ["full"] }` already in workspace deps. `opentelemetry*` crates are NOT yet in workspace deps — add them as optional under `[workspace.dependencies]`.
- `deny.toml` — `multiple-versions = "deny"` in `[bans]`. `opentelemetry-otlp` with `reqwest-client` would add reqwest 0.12 conflicting with the workspace's reqwest 0.13 (from jsonschema). Use `hyper-client` to avoid reqwest entirely.

## Constraints

- **deny.toml `multiple-versions = "deny"`**: Use `hyper-client` on opentelemetry-otlp — verified no reqwest dep with `cargo tree | grep reqwest` (returns empty). The `http-proto` feature uses `prost` for serialization, not tonic. No tonic version risk with this transport.
- **D007 (sync core)**: Business logic remains sync. OTel SDK export is async but contained to the tracing layer — `BatchSpanProcessor` handles async dispatch internally. `launch_agent` and `launch_agent_streaming` remain sync.
- **D001 (closures not traits)**: All OTel integration goes through `init_tracing()` free function and `#[cfg]` blocks. No OTel trait objects escape into business logic.
- **Feature flag scope**: `telemetry` feature on assay-core AND assay-cli. CLI enabling `assay-core/telemetry` transitively activates the OTel layer.
- **Default build must not pull OTel deps**: `cargo build -p assay-cli` (no features) must produce a tree with no opentelemetry crates. Verify with `cargo tree -p assay-cli | grep opentelemetry` (must be empty).
- **`tracing-subscriber` registry feature**: **Not yet added on this branch** — S05 must add `registry` to the workspace `tracing-subscriber` features (currently `["fmt", "env-filter"]`). S04 added it on the S04 branch; this branch does not have it. Required by `tracing-opentelemetry`.
- **TracingGuard shutdown**: `opentelemetry::global::shutdown_tracer_provider()` must be called on process exit to flush pending spans. Add to `TracingGuard::drop()` behind `#[cfg(feature = "telemetry")]` with a `tracing::warn!` on failure.
- **TRACEPARENT injection is feature-only**: `std::process::Command` inherits the parent environment automatically — no passthrough code needed when feature is disabled. The `.env("TRACEPARENT", ...)` call is only needed on the feature-enabled path to inject newly-computed context.

## Common Pitfalls

- **`opentelemetry-otlp` with `reqwest-client` feature causes dual-reqwest conflict** — Use `hyper-client` instead: `opentelemetry-otlp = { version = "0.31", default-features = false, features = ["http-proto", "hyper-client"] }`. Verified clean: `cargo add opentelemetry-otlp --no-default-features -F http-proto,hyper-client && cargo tree | grep reqwest` returns empty.
- **Missing `shutdown_tracer_provider()` on exit** — `BatchSpanProcessor` may not flush final batch before process exits. Add to `TracingGuard::drop()` behind cfg feature. This call is synchronous and blocks until flushed.
- **`tracing-opentelemetry` version misalignment** — `tracing-opentelemetry 0.32` requires `opentelemetry 0.31` and `opentelemetry_sdk 0.31`. All three must be the same generation. Mixing causes `TraceContextExt` not-found compile errors.
- **OTel layer added to subscriber but global propagator not set** — Call `opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new())` BEFORE subscriber init. Without it, `inject_context()` produces empty maps and TRACEPARENT is blank.
- **`features = ["rt-tokio"]` vs `features = ["rt-tokio-current-thread"]`** — Use `rt-tokio` (multi-thread). CLI uses `#[tokio::main]` which creates a multi-thread runtime. Wrong feature causes "no tokio runtime found" panic during provider initialization.
- **TRACEPARENT extraction when no active span** — `Span::current().is_disabled()` returns true when called outside any tracing span. Guard before extracting context and emit `tracing::debug!` when skipped so users can diagnose missing correlation:
  ```rust
  #[cfg(feature = "telemetry")]
  {
      let span = tracing::Span::current();
      if !span.is_disabled() {
          // inject TRACEPARENT via propagator
      } else {
          tracing::debug!("launch_agent: no active span; TRACEPARENT not injected");
      }
  }
  ```
- **Span ID format mismatch**: `SpanData.span_id` in the JSON file export (S04) uses the internal tracing `u64` span ID. W3C TRACEPARENT uses the OTel 16-hex-char span ID. These are parallel systems — do NOT try to unify them.
- **OTel init failure must be visible** — If OTel provider setup fails (bad endpoint format, missing runtime), `init_tracing()` must not silently fall back with zero indication. At minimum emit `tracing::warn!(endpoint, error, "OTLP init failed; trace export disabled")` and continue without the layer. A panic is also acceptable for clearly misconfigured endpoints if the CLI can catch it and print a clear message.
- **OTLP silent span drops** — `BatchSpanProcessor` silently discards spans when the endpoint is unreachable (internal TRACE-level log only). Users see no spans and no warning. Prescribe a startup warning: at minimum document that `RUST_LOG=opentelemetry=debug` will reveal drops; ideally add a `tracing::warn!` when the provider is initialized but endpoint is not reachable.

## Open Risks

- **cargo-deny hyper version**: `opentelemetry-http 0.31` pulls `hyper v1.8.x`. The workspace already has `hyper v1.8.x` (via rmcp → h2). Likely no conflict since same version — verify with `cargo deny check bans` after adding deps.
- **prost version alignment**: `http-proto` feature uses `prost` for protobuf. Check if prost is already in workspace dep tree (via rmcp or other crates) and at what version. If rmcp requires a different prost version, a deny.toml `skip` entry will be needed.
- **OTel SDK compile time**: ~50 new crates behind the feature flag. Not in CI default builds, but `cargo build --features telemetry` will be noticeably slower. Acceptable for an opt-in path.
- **`OTEL_EXPORTER_OTLP_ENDPOINT` vs config.toml**: The roadmap mentions both. Config.toml extension requires adding `telemetry: Option<TelemetryConfig>` to `Config` (with `deny_unknown_fields` — D092 pattern). Env-var-only is the MVP if scope is tight; config.toml is a follow-up.

## Integration Points

**Crate changes required:**
- `Cargo.toml` (workspace): add `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `tracing-opentelemetry` as optional workspace deps; add `registry` to `tracing-subscriber` features
- `crates/assay-core/Cargo.toml`: add `telemetry` feature enabling optional OTel deps
- `crates/assay-cli/Cargo.toml`: add `telemetry` feature enabling `assay-core/telemetry`
- `crates/assay-core/src/telemetry.rs`: add OTel layer init under `#[cfg(feature = "telemetry")]`; `TracingGuard::drop()` shutdown; `TracingConfig::otlp_endpoint` field
- `crates/assay-core/src/pipeline.rs`: inject TRACEPARENT in `launch_agent` (line 229 injection point) and `launch_agent_streaming` (line 337 function)

**Test strategy:**
- Feature-flag dep check (default): `cargo tree -p assay-cli | grep opentelemetry` — must return empty
- Feature-flag dep check (enabled): `cargo tree -p assay-cli --features telemetry | grep opentelemetry` — must show OTel tree
- TRACEPARENT injection test: subscriber with OTel layer + TraceContextPropagator, parent span active, spawn echo subprocess that prints env, assert `TRACEPARENT` appears in output
- OTel init failure test: pass invalid endpoint (e.g. `not-a-url`) — assert CLI exits with clear error or warn log, not a panic with no context
- Feature-gated `just ready`: `just ready` with default features must pass; `cargo build --features telemetry` must compile without errors
- End-to-end with Jaeger: UAT only — `cargo run --features telemetry -- run manifest.toml` with `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318` and local Jaeger

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| opentelemetry-otlp | none found | none found |
| tracing-opentelemetry | none found | none found |

## Sources

- `opentelemetry-otlp 0.31.1` transport verified: `cargo add opentelemetry-otlp --no-default-features -F http-proto,hyper-client && cargo tree | grep reqwest` returns empty — no reqwest dep with `hyper-client` feature
- `tracing-opentelemetry = "0.32.1"` requires `opentelemetry = "0.31"` and `opentelemetry_sdk = "0.31"` — version alignment verified by cargo search output
- `deny.toml` bans analysis: `multiple-versions = "deny"`; `http-proto + hyper-client` avoids the reqwest conflict; `prost` alignment is the remaining risk to verify at implementation time
- D143 (added to DECISIONS.md): supersedes D127 — `rt-tokio` with existing runtime is the chosen approach
- S04 Forward Intelligence: "S05 needs `otlp_endpoint: Option<String>` and the `telemetry` feature flag" — confirmed approach
- S01 Forward Intelligence: "`registry().with()` composition — S04/S05 add layers without changing call sites" — the `.with(otel_layer)` pattern is correct
