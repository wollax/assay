---
estimated_steps: 4
estimated_files: 3
---

# T04: Wire CLI endpoint config, verify feature-flag dep isolation, and run just ready

**Slice:** S05 — OTLP export and trace context propagation
**Milestone:** M009

## Description

Closes the integration loop. The CLI reads `OTEL_EXPORTER_OTLP_ENDPOINT` env var and populates `TracingConfig::otlp_endpoint` so the OTel layer activates when the env var is set. Adds dep-isolation assertions (default build has no OTel in tree). Runs `just ready` to verify full workspace green.

## Steps

1. In `crates/assay-cli/src/main.rs`, update `tracing_config_for()`:
   - After constructing the base TracingConfig, check `std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")`
   - If present, set `config.otlp_endpoint = Some(value)`
   - MCP config (warn level) also gets the endpoint if set — traces from MCP serve are valuable
2. Add dep-isolation verification to the test file or as a separate bash-based test:
   - `cargo tree -p assay-cli 2>/dev/null | grep -c opentelemetry` must output `0`
   - `cargo tree -p assay-cli --features telemetry 2>/dev/null | grep -c opentelemetry` must output a number > 0
   - These can be shell assertions in the test or documented as manual verification commands
3. Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` to catch any formatting/lint issues across the full workspace.
4. Run `just ready` — must pass completely (fmt, lint, test, deny all green). Fix any issues that surface.

## Must-Haves

- [ ] `tracing_config_for()` reads `OTEL_EXPORTER_OTLP_ENDPOINT` and sets `otlp_endpoint`
- [ ] `cargo tree -p assay-cli | grep opentelemetry` returns empty (default build dep isolation)
- [ ] `cargo build -p assay-cli --features telemetry` compiles successfully
- [ ] `just ready` passes with zero failures

## Verification

- `just ready` — full workspace green
- `cargo tree -p assay-cli | grep opentelemetry` — empty output (zero lines)
- `cargo tree -p assay-cli --features telemetry | grep opentelemetry` — non-empty output
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — all tests pass (final confirmation)

## Observability Impact

- Signals added/changed: CLI now activates OTel layer when `OTEL_EXPORTER_OTLP_ENDPOINT` is set — spans export to the configured collector
- How a future agent inspects this: set `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318` + run with `--features telemetry` to see traces in Jaeger; unset to disable
- Failure state exposed: If endpoint is set but feature not compiled in, otlp_endpoint is set on TracingConfig but ignored (no warn — this is expected behavior for default builds)

## Inputs

- `crates/assay-cli/src/main.rs` — tracing_config_for() at line 210
- T02 output — TracingConfig.otlp_endpoint field exists
- T03 output — TRACEPARENT injection working
- S05-RESEARCH.md — OTEL_EXPORTER_OTLP_ENDPOINT standard env var, default http://localhost:4318

## Expected Output

- `crates/assay-cli/src/main.rs` — tracing_config_for() reads OTEL_EXPORTER_OTLP_ENDPOINT
- Full workspace passing `just ready`
- Verified dep isolation (default build has no OTel deps)
