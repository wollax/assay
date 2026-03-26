# Kata State

**Active Milestone:** M009 — Observability
**Active Slice:** S05 — OTLP export and trace context propagation
**Active Task:** T01 — Add OTel workspace deps, feature flags, and red-state integration tests
**Phase:** Executing

## Recent Decisions
- D143: D127 superseded — use rt-tokio with existing runtime, no scoped runtime
- D144: http-proto + hyper-client transport for opentelemetry-otlp (avoids reqwest conflict)
- D145: S05 test-first contract + dep isolation assertions

## Blockers
- None

## Next Action
Execute T01: Add OTel workspace deps (opentelemetry 0.31, opentelemetry_sdk 0.31, opentelemetry-otlp 0.31, tracing-opentelemetry 0.32) as optional workspace deps; add `telemetry` feature to assay-core and assay-cli; add `registry` to tracing-subscriber features; create red-state integration tests in `crates/assay-core/tests/telemetry_otlp.rs`; verify `cargo deny check bans` passes.
