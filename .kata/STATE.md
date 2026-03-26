# Kata State

**Active Milestone:** M009 — Observability
**Active Slice:** S05 — OTLP export and trace context propagation
**Active Task:** T02 — Implement OTel tracing layer in init_tracing() with feature-flagged TracingGuard shutdown
**Phase:** Executing

## Recent Decisions
- D143: D127 superseded — use rt-tokio with existing runtime, no scoped runtime
- D144: http-proto + hyper-client transport for opentelemetry-otlp (avoids reqwest conflict)
- D145: S05 test-first contract + dep isolation assertions

## Blockers
- None

## Next Action
Execute T02: Add `otlp_endpoint: Option<String>` to TracingConfig, wire OTel tracing layer behind `#[cfg(feature = "telemetry")]` in init_tracing(), add TracingGuard shutdown, graceful degradation on init failure.
