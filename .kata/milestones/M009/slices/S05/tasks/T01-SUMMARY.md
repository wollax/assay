---
id: T01
parent: S05
milestone: M009
provides:
  - telemetry feature flag on assay-core enabling 4 optional OTel deps
  - telemetry feature flag on assay-cli forwarding to assay-core/telemetry
  - registry feature added to tracing-subscriber workspace dep
  - OTel workspace deps (opentelemetry 0.31, opentelemetry_sdk 0.31, opentelemetry-otlp 0.31, tracing-opentelemetry 0.32)
  - Red-state integration test file defining OTLP and TRACEPARENT contracts
key_files:
  - Cargo.toml
  - crates/assay-core/Cargo.toml
  - crates/assay-cli/Cargo.toml
  - crates/assay-core/tests/telemetry_otlp.rs
key_decisions:
  - "OTel deps use http-proto + hyper-client transport (no reqwest) to avoid deny.toml version conflicts"
  - "optional = true at crate level only; workspace.dependencies defines version+features, crates opt in"
patterns_established:
  - "Feature flag forwarding: assay-cli/telemetry enables assay-core/telemetry"
  - "Red-state tests compile but fail at runtime to define contracts for future tasks"
observability_surfaces:
  - "cargo tree -p assay-core --features telemetry | grep opentelemetry — shows OTel dep tree"
  - "cargo tree -p assay-cli | grep opentelemetry — empty confirms default build isolation"
duration: 10min
verification_result: passed
completed_at: 2026-03-26T12:00:00Z
blocker_discovered: false
---

# T01: Add OTel workspace deps, feature flags, and red-state integration tests

**OTel 0.31 workspace deps with http-proto transport, telemetry feature flags on assay-core/assay-cli, and red-state integration tests defining OTLP init and TRACEPARENT injection contracts**

## What Happened

Added four OpenTelemetry workspace dependencies (opentelemetry 0.31, opentelemetry_sdk 0.31 with rt-tokio, opentelemetry-otlp 0.31 with http-proto+hyper-client, tracing-opentelemetry 0.32) and the `registry` feature to tracing-subscriber. Created `telemetry` feature on assay-core that enables all four OTel deps as optional, and a forwarding `telemetry` feature on assay-cli. Default builds pull zero OTel dependencies.

Created `telemetry_otlp.rs` integration test file with two red-state tests: `test_otel_layer_init_compiles` (passes — verifies OTel types are linkable) and `test_traceparent_injected_in_subprocess` (fails — asserts TRACEPARENT env var injection that T03 will implement).

## Verification

- `cargo build -p assay-cli` — default build compiles, no OTel pulled in ✓
- `cargo build -p assay-core --features telemetry` — feature-flagged build compiles ✓
- `cargo build -p assay-cli --features telemetry` — CLI with telemetry compiles ✓
- `cargo deny check bans` — no new violations ✓
- `cargo tree -p assay-cli | grep opentelemetry` — empty (default isolation) ✓
- `cargo tree -p assay-cli --features telemetry | grep opentelemetry` — shows full OTel tree ✓
- `cargo test -p assay-core --test telemetry_otlp --features telemetry --no-run` — compiles ✓
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — 1/2 pass, 1/2 red state (expected) ✓

## Diagnostics

- `cargo tree -p assay-core --features telemetry | grep opentelemetry` — inspect OTel dep tree
- Without `--features telemetry`, same command returns empty — confirms zero-cost default

## Deviations

Task plan specified `optional = true` at workspace level in root Cargo.toml. Cargo does not support `optional` on workspace dependencies — only on crate-level deps. Fixed by removing `optional` from workspace entries and keeping `optional = true` on the crate-level dep declarations in assay-core's Cargo.toml.

First test (`test_otel_layer_init_compiles`) was adapted to not reference the not-yet-existing `otlp_endpoint` field. Instead it verifies OTel types are linkable under the telemetry feature. The `otlp_endpoint`-based init test will be updated when T02 adds the field.

## Known Issues

None.

## Files Created/Modified

- `Cargo.toml` — Added 4 OTel workspace deps + registry feature on tracing-subscriber
- `crates/assay-core/Cargo.toml` — Added telemetry feature + 4 optional OTel deps
- `crates/assay-cli/Cargo.toml` — Added telemetry feature forwarding to assay-core/telemetry
- `crates/assay-core/tests/telemetry_otlp.rs` — Red-state integration tests (2 tests)
