---
id: T04
parent: S05
milestone: M009
provides:
  - CLI reads OTEL_EXPORTER_OTLP_ENDPOINT env var and populates TracingConfig.otlp_endpoint
  - Both default and MCP tracing configs receive the endpoint when env var is set
  - Verified dep isolation: default build has 0 OTel deps, telemetry feature adds 13
  - Full workspace green via just ready (fmt, lint, test, deny)
key_files:
  - crates/assay-cli/src/main.rs
key_decisions:
  - "OTEL_EXPORTER_OTLP_ENDPOINT applied after config variant selection so both default and MCP configs get it — MCP serve traces are valuable for debugging agent subprocess issues"
patterns_established:
  - "Env-var-driven OTel activation: set OTEL_EXPORTER_OTLP_ENDPOINT to enable, unset to disable — zero code changes needed"
observability_surfaces:
  - "Set OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 + build with --features telemetry to see traces in Jaeger/Tempo"
  - "cargo tree -p assay-cli | grep opentelemetry — must return empty for default build"
duration: 8min
verification_result: passed
completed_at: 2026-03-26T12:00:00Z
blocker_discovered: false
---

# T04: Wire CLI endpoint config, verify feature-flag dep isolation, and run just ready

**CLI reads OTEL_EXPORTER_OTLP_ENDPOINT env var to activate OTLP export, with verified dep isolation and full workspace green**

## What Happened

Updated `tracing_config_for()` in `crates/assay-cli/src/main.rs` to read
`OTEL_EXPORTER_OTLP_ENDPOINT` from the environment after constructing the
base TracingConfig variant. The endpoint is applied to both the default
config and the MCP config, so traces from `assay mcp serve` are captured
when a collector is available.

The change is minimal: a single `std::env::var` check after the
if/else that selects the config variant, setting `config.otlp_endpoint`
when the env var is present.

Dep isolation was verified: `cargo tree -p assay-cli` shows zero
opentelemetry crates in the default build, while `--features telemetry`
pulls in 13 OTel crates. `just ready` passes completely (fmt, clippy,
all tests, cargo-deny).

## Verification

| Check | Result |
|-------|--------|
| `cargo tree -p assay-cli \| grep -c opentelemetry` | 0 (dep isolation ✓) |
| `cargo tree -p assay-cli --features telemetry \| grep -c opentelemetry` | 13 (feature enables OTel ✓) |
| `cargo fmt --all -- --check` | clean ✓ |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean ✓ |
| `cargo test -p assay-core --test telemetry_otlp --features telemetry` | 2/2 passed ✓ |
| `just ready` | All checks passed ✓ |

## Diagnostics

- Set `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318` and build with `--features telemetry` to see traces in a collector
- Without the env var, `otlp_endpoint` is `None` and the OTel layer is skipped
- Without `--features telemetry`, the `otlp_endpoint` field is set but ignored (no OTel code compiled in) — this is expected behavior

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/main.rs` — `tracing_config_for()` now reads `OTEL_EXPORTER_OTLP_ENDPOINT` and sets `otlp_endpoint` on both default and MCP configs
