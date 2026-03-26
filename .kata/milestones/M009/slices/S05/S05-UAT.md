# S05: OTLP export and trace context propagation — UAT

**Milestone:** M009
**Written:** 2026-03-26

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: OTLP export requires a running collector (Jaeger/Tempo) to verify traces arrive with correct parent-child relationships — this cannot be tested with artifact inspection alone

## Preconditions

- Docker installed and running
- Jaeger all-in-one container available: `docker run -d --name jaeger -p 16686:16686 -p 4318:4318 jaegertracing/all-in-one:latest`
- Project built with telemetry feature: `cargo build -p assay-cli --features telemetry`
- A valid spec exists in `.assay/specs/` (e.g. from `assay init` + `assay plan`)

## Smoke Test

Run `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 cargo run -p assay-cli --features telemetry -- gate run <spec-slug>` and check Jaeger UI at `http://localhost:16686` for a trace with pipeline spans.

## Test Cases

### 1. OTLP traces appear in Jaeger for a pipeline run

1. Start Jaeger: `docker run -d --name jaeger -p 16686:16686 -p 4318:4318 jaegertracing/all-in-one:latest`
2. Run: `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 cargo run -p assay-cli --features telemetry -- gate run <spec-slug>`
3. Open `http://localhost:16686` in browser
4. Select service "assay" (or the service name set by OTel) from the dropdown
5. Click "Find Traces"
6. **Expected:** A trace appears with pipeline stage spans (spec_load, worktree_create, agent_launch, gate_eval) nested under a run_session root span, each with timing data

### 2. Orchestration produces nested span tree

1. Create a multi-session manifest with 2-3 sessions
2. Run: `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 cargo run -p assay-cli --features telemetry -- run manifest.toml`
3. Check Jaeger UI for traces
4. **Expected:** Orchestration root span → per-session child spans → pipeline stage spans within each session. Merge runner spans visible if merges occur.

### 3. Default build has no OTel deps

1. Run: `cargo tree -p assay-cli | grep opentelemetry`
2. **Expected:** Empty output (zero OTel crates in default dep tree)

### 4. Graceful degradation on bad endpoint

1. Run: `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:9999 RUST_LOG=warn cargo run -p assay-cli --features telemetry -- gate run <spec-slug>`
2. **Expected:** Warning log `OTLP exporter init failed; trace export disabled` appears on stderr. Command continues and completes normally without OTel traces.

### 5. TRACEPARENT visible in subprocess environment

1. Run: `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 RUST_LOG=debug cargo run -p assay-cli --features telemetry -- gate run <spec-slug>`
2. **Expected:** Debug logs show TRACEPARENT being injected into agent subprocess. If agent supports OTel, its spans appear as children of the pipeline span in Jaeger.

## Edge Cases

### No OTEL_EXPORTER_OTLP_ENDPOINT set

1. Run: `cargo run -p assay-cli --features telemetry -- gate run <spec-slug>`
2. **Expected:** No OTel layer initialized. Pipeline runs normally. No warnings about missing endpoint. JSON file traces (S04) still written.

### Built without telemetry feature

1. Run: `cargo run -p assay-cli -- gate run <spec-slug>`
2. **Expected:** No OTel code executes. OTEL_EXPORTER_OTLP_ENDPOINT is ignored even if set. Pipeline runs normally with fmt tracing only.

## Failure Signals

- No traces in Jaeger after a pipeline run with OTEL_EXPORTER_OTLP_ENDPOINT set → exporter init may have silently failed (check RUST_LOG=warn)
- `cargo build` (default) pulls in opentelemetry crates → feature flag isolation broken
- Agent subprocess traces disconnected from parent → TRACEPARENT injection not working (check RUST_LOG=debug for injection logs)
- Compile error with `--features telemetry` → OTel dep version mismatch or missing feature flags

## Requirements Proved By This UAT

- R064 — OTLP trace export: UAT proves traces actually arrive at Jaeger with correct span structure (contract tests only prove compilation and init)
- R065 — Trace context propagation: UAT proves child process traces correlate with parent orchestration in Jaeger UI (contract tests prove TRACEPARENT env var format)
- R027 — OpenTelemetry instrumentation (final validation): UAT proves the full trace tree — pipeline stages, orchestration nesting, and cross-process correlation — is visible end-to-end in Jaeger

## Not Proven By This UAT

- Metric collection (R067 — deferred)
- TUI trace viewer (R066 — deferred)
- Long-running production stability of the OTLP exporter (BatchSpanProcessor behavior under load)
- Multiple concurrent orchestration runs producing correctly isolated traces

## Notes for Tester

- Jaeger all-in-one uses port 4318 for OTLP HTTP. If using Grafana Tempo or another collector, adjust the endpoint accordingly.
- The service name in Jaeger may appear as `unknown_service` unless `OTEL_SERVICE_NAME=assay` is also set — this is cosmetic.
- OTel TracingGuard flush happens on drop — if the process is killed (SIGKILL), pending spans may not be exported. Normal exit (including Ctrl+C with SIGINT) should flush correctly.
- The `eprintln!` in TracingGuard::drop() on shutdown error is intentional — can't use tracing inside the tracing infrastructure's shutdown path.
