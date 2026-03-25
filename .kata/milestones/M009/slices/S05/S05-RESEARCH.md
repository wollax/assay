# S05: OTLP export and trace context propagation — Research

**Researched:** 2026-03-25
**Domain:** OpenTelemetry OTLP export, W3C Trace Context propagation, feature-flagged Cargo deps
**Confidence:** HIGH

## Summary

S05 delivers the production-grade observability path: OTLP export to Jaeger/Grafana Tempo behind a `telemetry` feature flag, and W3C TRACEPARENT env var injection into subprocess spawns so agent traces correlate with the parent orchestration trace.

The key finding is that tokio is **already a direct dependency of assay-core** (via `rmcp` and MCP server hosting), so the "scoped tokio runtime for async OTLP export" concern from D127 is partially solved — we don't need a hidden new runtime. However, the OTel SDK deps (opentelemetry, opentelemetry_sdk, tracing-opentelemetry) are large and must be feature-gated to avoid bloating the default build. The TRACEPARENT injection is a simple 2-line change in `launch_agent` and `launch_agent_streaming`.

**Primary recommendation:** Add a `telemetry` feature on assay-core and assay-cli. Use `opentelemetry-otlp` with `default-features = false, features = ["http-proto", "hyper-client"]` to avoid the reqwest version conflict (workspace uses reqwest 0.13 from jsonschema dev-dep; opentelemetry-http ships reqwest 0.12 — conflict with `deny.toml multiple-versions = "deny"`). Use `opentelemetry_sdk = { features = ["rt-tokio"] }` and `tracing-opentelemetry = "0.32"`. Add `opentelemetry_sdk::propagation::TraceContextPropagator` for TRACEPARENT extraction. All OTel deps are `optional` and activated only under `#[cfg(feature = "telemetry")]` blocks.

## Recommendation

**Feature flag approach: two-layer opt-in**

1. `assay-core/Cargo.toml` adds `telemetry` feature that enables optional deps: `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `tracing-opentelemetry`.
2. `assay-cli/Cargo.toml` adds `telemetry` feature that enables `assay-core/telemetry`.
3. `#[cfg(feature = "telemetry")]` guards all OTel code in `telemetry.rs`, `pipeline.rs`, and `launch_agent` / `launch_agent_streaming`.
4. `TracingConfig` gains `otlp_endpoint: Option<String>` (always-present field, always-None by default; only acted on when `telemetry` feature is compiled in).
5. `TRACEPARENT` injection in `launch_agent` / `launch_agent_streaming`: extract current span context via `tracing::Span::current()` + `opentelemetry::global::get_text_map_propagator` + inject into `Command::env("TRACEPARENT", ...)`. When feature is disabled, no env var injection happens.

**Transport:** `http-proto + hyper-client` (not grpc-tonic, not reqwest-client). This avoids the reqwest version conflict and avoids pulling TLS crates into the default dep tree. The OTLP endpoint defaults to `http://localhost:4318` (standard Jaeger/Tempo HTTP port).

**Scoped tokio runtime:** Since tokio is already in assay-core (rt-multi-thread via rmcp), use `tokio::runtime::Builder::new_current_thread().enable_all().build()` scoped to the OTel provider initialization in `init_tracing()` — or simpler: use the existing tokio dependency's `#[tokio::main]` in CLI main and let the OTel SDK use the existing runtime via `rt-tokio` feature. No new runtime overhead.

**TRACEPARENT extraction without OTel SDK (fallback):** When `telemetry` feature is disabled, the TRACEPARENT env var should still be passed through if already set in the parent environment. Simple: `if let Ok(tp) = std::env::var("TRACEPARENT") { cmd.env("TRACEPARENT", tp); }`. This costs zero deps and allows manual propagation.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Bridging tracing spans to OTel SDK | `tracing-opentelemetry = "0.32"` layer | Direct integration — one `.with(otel_layer)` in `init_tracing()`; maintains parent-child span relationships |
| W3C TRACEPARENT serialization | `opentelemetry_sdk::propagation::TraceContextPropagator` | Standard compliant; handles the `00-traceid-spanid-flags` format correctly |
| OTLP HTTP/protobuf export | `opentelemetry-otlp` `http-proto` + `hyper-client` features | Avoids reqwest version conflict; no TLS overhead; ~200 dep lines |
| Async OTLP flush | `opentelemetry_sdk` `rt-tokio` feature | Reuses existing tokio dep; `shutdown_tracer_provider()` flushes in-flight spans on process exit |

## Existing Code and Patterns

- `crates/assay-core/src/telemetry.rs` — Contains `init_tracing(TracingConfig) -> TracingGuard`. The `registry().with(filter).with(fmt_layer).with(json_layer)` chain is the extension point: add `.with(otel_layer)` here. `TracingConfig` already has `traces_dir: Option<PathBuf>` — add `otlp_endpoint: Option<String>` with the same pattern.
- `crates/assay-core/src/pipeline.rs` — `launch_agent()` at line 229: `Command::new("claude").args(...).current_dir(...)` — this is where `.env("TRACEPARENT", traceparent)` is injected. Same for `launch_agent_streaming()` at line 360.
- `crates/assay-cli/src/main.rs` — `tracing_config_for()` returns `TracingConfig` per subcommand. S05 adds logic to check `OTEL_EXPORTER_OTLP_ENDPOINT` env var and populate `TracingConfig::otlp_endpoint` when set.
- `Cargo.toml` workspace — `tokio = { version = "1", features = ["full"] }` already in workspace deps. `opentelemetry*` crates are NOT yet in workspace deps — add them as optional under `[workspace.dependencies]`.
- `deny.toml` — `multiple-versions = "deny"` in `[bans]`. reqwest 0.12 (from `opentelemetry-http`) vs reqwest 0.13 (from jsonschema dev-dep) will conflict. Use `hyper-client` feature on opentelemetry-otlp to avoid reqwest entirely. Must verify cargo-deny passes after adding deps.

## Constraints

- **deny.toml `multiple-versions = "deny"`**: `opentelemetry-otlp` with `reqwest-client` feature pulls reqwest 0.12; workspace already has reqwest 0.13 (jsonschema dev-dep). Use `hyper-client` instead — no reqwest, no conflict.
- **D007 (sync core)**: Business logic must remain sync. OTel SDK export is async but contained to the tracing layer — the SDK's `BatchSpanProcessor` handles async dispatch internally. `launch_agent` and `launch_agent_streaming` remain sync.
- **D001 (closures not traits)**: All OTel integration goes through `init_tracing()` free function and `#[cfg]` blocks. No OTel trait objects escape into business logic.
- **Feature flag scope**: `telemetry` feature must be on assay-core (where spans originate) AND assay-cli (binary that calls init_tracing). The feature propagates transitively — CLI enabling `assay-core/telemetry` is sufficient for the OTel layer to compile.
- **Default build must not pull OTel deps**: `cargo build -p assay-cli` (no features) must not include opentelemetry in the dep tree. Verify with `cargo tree -p assay-cli | grep opentelemetry` (should be empty with default features).
- **TracingGuard shutdown**: `opentelemetry::global::shutdown_tracer_provider()` must be called on program exit to flush pending spans. Add to `TracingGuard::drop()` behind `#[cfg(feature = "telemetry")]`.
- **`tracing-subscriber` registry feature**: Already added in S04 (`registry = true` in workspace Cargo.toml). The `tracing-opentelemetry` layer requires `registry` feature — already satisfied.

## Common Pitfalls

- **`opentelemetry-otlp` with `reqwest-client` feature causes dual-reqwest conflict** — Use `hyper-client` feature instead: `opentelemetry-otlp = { version = "0.31", default-features = false, features = ["http-proto", "hyper-client"] }`. Verified this builds clean in a test crate with no reqwest dep added.
- **Missing `shutdown_tracer_provider()` on exit** — Without explicit shutdown, the `BatchSpanProcessor` may not flush its final batch before the process exits. Add `opentelemetry::global::shutdown_tracer_provider()` to `TracingGuard::drop()` (behind cfg feature). This is synchronous — it blocks until flushed.
- **`tracing-opentelemetry` version misalignment** — `tracing-opentelemetry 0.32` requires `opentelemetry 0.31` and `opentelemetry_sdk 0.31`. All three must be pinned to the same generation (currently 0.31/0.31/0.32). Mixing versions (e.g. tracing-opentelemetry 0.29 with opentelemetry 0.31) causes compile errors with `TraceContextExt` not found.
- **OTel layer added to subscriber but global propagator not set** — `tracing-opentelemetry` requires `opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new())` to be called BEFORE the subscriber is initialized. Without it, `inject_context()` produces empty maps and TRACEPARENT is blank.
- **`features = ["rt-tokio"]` vs `features = ["rt-tokio-current-thread"]`** — Use `rt-tokio` (multi-thread) not `rt-tokio-current-thread` since the CLI uses `tokio::main` (multi-thread runtime). Wrong feature causes "no tokio runtime found" panic during provider initialization.
- **TRACEPARENT extraction: `Span::current()` returns a disabled span when called outside a tracing span** — In `launch_agent`, the call happens inside `#[instrument]`-decorated function so `Span::current()` is always valid. But in `launch_agent_streaming`, the caller may not always be in a span. Guard with `!Span::current().is_disabled()` before extracting TRACEPARENT.
- **Span ID format mismatch**: `SpanData.span_id` in the JSON file export (S04) uses the internal tracing span ID (`u64`). The W3C TRACEPARENT format uses the OTel 16-hex-char span ID (different). These are parallel systems — JSON file export uses the tracing layer's IDs; OTLP export uses the OTel layer's IDs. They are not the same. Do NOT try to unify them.

## Open Risks

- **cargo-deny version conflict**: Even with `hyper-client`, `opentelemetry-http 0.31` pulls in `hyper v1.8.x`. The workspace already has `hyper v1.8.x` (via rmcp → h2). Verify `cargo deny check bans` passes — likely no conflict since it's the same version, but must be tested explicitly.
- **tonic version duplication**: `opentelemetry-otlp http-proto` still pulls `tonic 0.14.x` for internal protobuf support. Check if tonic is already in the workspace dep tree (via rmcp). If rmcp uses a different tonic version, deny.toml skip entry needed.
- **OTel SDK size impact on compile time**: opentelemetry_sdk adds ~50 crates. Feature-gated so not in CI default builds, but developers running `cargo build --features telemetry` will see slower builds. Acceptable since this is an opt-in path.
- **`OTEL_EXPORTER_OTLP_ENDPOINT` vs config.toml**: The roadmap mentions both env var AND config.toml endpoint configuration. Config.toml extension requires adding `telemetry: Option<TelemetryConfig>` to `Config` struct (with `deny_unknown_fields` — must use D092 pattern). If scope is tight, env-var-only is the MVP.

## Integration Points

**Crate changes required:**
- `Cargo.toml` (workspace): add `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `tracing-opentelemetry` as optional workspace deps
- `crates/assay-core/Cargo.toml`: add `telemetry` feature enabling optional OTel deps
- `crates/assay-cli/Cargo.toml`: add `telemetry` feature enabling `assay-core/telemetry`
- `crates/assay-core/src/telemetry.rs`: add OTel layer init under `#[cfg(feature = "telemetry")]`
- `crates/assay-core/src/pipeline.rs`: inject TRACEPARENT in `launch_agent` and `launch_agent_streaming`

**Test strategy:**
- Feature-flag dep check: `cargo tree -p assay-cli | grep opentelemetry` with default features — should be empty
- Feature-flag dep check with feature: `cargo tree -p assay-cli --features telemetry | grep opentelemetry` — should show the full OTel tree
- TRACEPARENT integration test: set up subscriber with OTel layer, create a parent span, call a mock `launch_agent`-like function, assert `TRACEPARENT` env var is set on the spawned command (capture via test env spy)
- End-to-end: `cargo run --features telemetry -- run manifest.toml` with `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318` and a local Jaeger instance — UAT only

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| opentelemetry-otlp | none found | none found |
| tracing-opentelemetry | none found | none found |

## Sources

- `opentelemetry-otlp v0.31.1` features verified by test crate at `/tmp/otel_http_proto` — `http-proto + hyper-client` builds with no reqwest dep (confirmed: `cargo tree | grep reqwest` returns empty)
- `tracing-opentelemetry = "0.32.1"` requires `opentelemetry = "0.31"` and `opentelemetry_sdk = "0.31"` — version alignment verified by cargo search
- `deny.toml` analysis: `multiple-versions = "deny"` applies to reqwest; hyper-client avoids the conflict; tonic version needs verification at implementation time
- S04 Forward Intelligence: "S05 needs to add `otlp_endpoint: Option<String>` or similar and gate the OTel layer behind the `telemetry` feature flag" — confirmed as the approach
- S01 Forward Intelligence: "`registry().with()` composition so downstream slices can add JSON file and OTLP layers without changing call sites" — the `.with(otel_layer)` approach is correct
