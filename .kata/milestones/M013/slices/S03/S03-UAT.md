# S03: OTel Metrics — UAT

**Milestone:** M013
**Written:** 2026-03-28

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: Contract tests prove the metrics API compiles, counters increment, and histograms record correctly in-process. Live-runtime UAT adds the only thing contract tests cannot: verifying the OTLP export pipeline actually delivers data to a real collector. The gap is narrow but real — the export path involves HTTP transport, periodic flushing, and collector ingestion that are outside the in-process test boundary.

## Preconditions

1. Build with telemetry feature: `cargo build --features telemetry`
2. Start a local OTLP collector (Jaeger all-in-one or Grafana Tempo):
   - Jaeger: `docker run -p 4318:4318 -p 16686:16686 jaegertracing/all-in-one:latest`
   - Tempo: see Grafana Tempo quickstart
3. Set endpoint: `export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318`
4. Have a valid `.assay/` project with a spec and gates.toml

## Smoke Test

Run `gate run` on any spec and check that `assay.sessions.launched` counter appears in Jaeger/Tempo UI within 30 seconds.

## Test Cases

### 1. Counter increment on gate run

1. Run: `cargo run --features telemetry -- gate run <spec-slug>`
2. Open Jaeger UI at `http://localhost:16686`
3. Search for service `assay`, metrics endpoint
4. **Expected:** `assay.sessions.launched` counter = 1; `assay.gates.evaluated` counter ≥ 1

### 2. Histogram records gate eval latency

1. Run: `cargo run --features telemetry -- gate run <spec-slug>` (any spec with ≥1 criterion)
2. In collector UI, find `assay.gate_eval.latency_ms` histogram
3. **Expected:** Histogram has ≥1 data point with a positive millisecond value

### 3. Merge counter increments on orchestrate run

1. Run: `cargo run --features telemetry -- orchestrate run <manifest.toml>` (multi-session manifest)
2. **Expected:** `assay.merges.attempted` counter = 1 after run completes

### 4. Default build has zero new OTel deps

1. Run: `cargo tree -p assay-core 2>&1 | grep opentelemetry_sdk`
2. **Expected:** No output (opentelemetry_sdk absent from default build)

### 5. Clean shutdown — no hang

1. Run: `cargo run --features telemetry -- gate run <spec-slug>`
2. Wait for completion
3. **Expected:** Process exits cleanly within 5 seconds after gate run finishes (meter provider flush + tracer flush complete)

## Edge Cases

### Missing OTLP endpoint

1. Unset `OTEL_EXPORTER_OTLP_ENDPOINT`
2. Run: `cargo run --features telemetry -- gate run <spec-slug>`
3. **Expected:** Process completes normally (graceful degradation); `tracing::debug!` or no log entry about metrics init; no crash

### Collector unreachable

1. Set `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:9999` (nothing listening)
2. Run: `cargo run --features telemetry -- gate run <spec-slug>`
3. **Expected:** Process completes normally despite export failures; possible `tracing::warn!` about export failure; no panic

## Failure Signals

- Process hangs at exit (meter/tracer provider not flushing — check D179 shutdown order in `TracingGuard::drop`)
- `tracing::warn!` about metrics init failure when endpoint IS configured (indicates transport init problem)
- Zero metrics in collector after `gate run` with endpoint set (export path broken — check OTLP HTTP transport)
- `cargo build -p assay-core` (no features) fails or pulls in OTel deps (feature isolation broken)

## Requirements Proved By This UAT

- R067 — Real OTLP collector receiving `assay.sessions.launched`, `assay.gates.evaluated`, `assay.merges.attempted` counters and `assay.gate_eval.latency_ms`, `assay.agent_run.duration_ms` histograms proves end-to-end metrics export pipeline functions correctly.

## Not Proven By This UAT

- Metric accuracy under concurrent orchestration (multiple sessions in parallel) — not tested here
- Attribute/label correctness on metric data points (currently empty `&[]`, so there's nothing to verify)
- Long-running metric accumulation (counter values across multiple `gate run` invocations) — periodic exporter semantics not validated
- Grafana dashboard creation or alerting configuration — out of scope for Assay itself

## Notes for Tester

- The periodic exporter has a default flush interval (typically 60s). Use Jaeger's "last 1 minute" search window or wait for the interval before expecting metrics to appear.
- `cargo run --features telemetry` builds the assay-cli binary with telemetry. Make sure you're using the telemetry-enabled binary throughout.
- If using Tempo, metrics OTLP and traces OTLP share the same port (4318) in the default all-in-one config.
- The `assay.agent_run.duration_ms` histogram only populates when an agent actually runs (not on dry-run gate evaluations that short-circuit without launching a subprocess).
