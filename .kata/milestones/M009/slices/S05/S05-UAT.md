# S05: OTLP export and trace context propagation — UAT

**Milestone:** M009
**Written:** 2026-03-26

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: Dep isolation and TRACEPARENT injection are contract-tested automatically. The only gap that requires a human tester is Jaeger UI visualization — verifying that spans appear with correct parent-child nesting requires a running collector and a human to visually inspect the trace tree.

## Preconditions

1. Docker is available and running.
2. Build with telemetry feature: `cargo build --features telemetry --release` or use `cargo run --features telemetry`.
3. A test manifest exists (e.g. `test-manifest.toml`) with at least one session entry.

## Smoke Test

Run `cargo tree -p assay-cli | grep opentelemetry` — must return empty. This confirms the default build has zero OTel deps. If non-empty, the feature flag isolation has broken.

## Test Cases

### 1. Default build has no OTel deps

1. `cargo build -p assay-cli`
2. `cargo tree -p assay-cli | grep opentelemetry`
3. **Expected:** no output (zero OTel crates in default dep tree)

### 2. Telemetry feature build compiles

1. `cargo build -p assay-cli --features telemetry`
2. **Expected:** compiles successfully with no errors; warnings allowed but not errors

### 3. OTLP traces appear in Jaeger

1. Start Jaeger: `docker run --rm -p 4318:4318 -p 16686:16686 jaegertracing/all-in-one:latest`
2. Set endpoint: `export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318`
3. Run a pipeline: `cargo run --features telemetry -- run test-manifest.toml` (any manifest with a real or mock session)
4. Open `http://localhost:16686` in a browser
5. Select the `assay` service in the Jaeger search panel
6. Find the most recent trace
7. **Expected:** Trace tree shows `pipeline::run_session` as root span with child spans for each pipeline stage (spec_load, worktree_create, agent_launch, gate_eval, merge_propose). Spans have correct timing and fields (spec_slug, stage).

### 4. OTel init failure degrades gracefully

1. Set an invalid endpoint: `export OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:1` (nothing listening)
2. Run: `RUST_LOG=warn cargo run --features telemetry -- run test-manifest.toml`
3. **Expected:** A `WARN` line on stderr containing `OTLP exporter init failed; trace export disabled` and the endpoint URL. The run continues normally (does not crash or hang).

### 5. TRACEPARENT injected into child processes

1. `cargo test -p assay-core --test telemetry_otlp --features telemetry -- --nocapture`
2. **Expected:** Both tests pass: `test_otel_layer_init_compiles` and `test_traceparent_injected_in_subprocess`. The subprocess output test confirms a valid W3C traceparent string (`00-<32hex>-<16hex>-<2hex>`) appears in the child process environment.

## Edge Cases

### No OTEL_EXPORTER_OTLP_ENDPOINT set (default behavior)

1. Ensure `OTEL_EXPORTER_OTLP_ENDPOINT` is not set in the shell.
2. Run `cargo run --features telemetry -- run test-manifest.toml`
3. **Expected:** No OTLP-related output; run completes normally. `otlp_endpoint` is `None`, OTel layer is skipped.

### Telemetry feature disabled (default build)

1. Run `cargo run -- run test-manifest.toml` (no `--features telemetry`)
2. Set `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318`
3. **Expected:** `otlp_endpoint` field is set in TracingConfig but no OTel code runs (cfg-gated). No OTLP connection is made. Run completes normally.

## Failure Signals

- `cargo tree -p assay-cli | grep opentelemetry` returns output — dep isolation broken
- `cargo build -p assay-cli --features telemetry` fails to compile — OTel integration broken
- No `assay` service appears in Jaeger — OTLP export not reaching collector (check endpoint, check for warn log)
- Trace tree in Jaeger shows flat spans (no nesting) — parent-child propagation broken
- Process hangs on exit — TracingGuard::drop() is blocking on flush; check RUST_LOG=opentelemetry=debug

## Requirements Proved By This UAT

- R064 (OTLP trace export) — Jaeger UI shows spans from an instrumented pipeline run, proving end-to-end OTLP export works with a real collector
- R065 (Trace context propagation) — TRACEPARENT test case proves W3C trace context appears in subprocess environment

## Not Proven By This UAT

- Correct parent-child nesting in a multi-session DAG orchestration run (requires a real multi-session manifest and Jaeger; automated tests cover span names but not visual nesting in Jaeger UI)
- OTLP export to Grafana Tempo (tested only with Jaeger; protocol is identical but endpoint differs)
- Spans from within an actual Claude Code agent process being correlated back to the parent orchestration trace (requires a live agent run with OTel instrumented harness)
- Performance impact of BatchSpanProcessor under high-throughput orchestration runs

## Notes for Tester

- The Jaeger `all-in-one` image is the simplest way to test locally. It accepts OTLP HTTP on port 4318 and serves the UI on 16686.
- OTLP uses `http-proto` (HTTP/1.1 + protobuf), not gRPC. If your collector only accepts gRPC, it will not receive spans.
- If no spans appear in Jaeger: (1) check `RUST_LOG=warn` output for the init failure warning, (2) check `RUST_LOG=opentelemetry=debug` for BatchSpanProcessor export attempts, (3) confirm the endpoint URL matches the collector's HTTP OTLP port (default 4318).
- The `assay mcp serve` path also receives the `otlp_endpoint` from the env var — traces from MCP tool invocations are captured when a collector is running.
