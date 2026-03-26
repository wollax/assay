---
estimated_steps: 5
estimated_files: 5
---

# T01: Add OTel workspace deps, feature flags, and red-state integration tests

**Slice:** S05 — OTLP export and trace context propagation
**Milestone:** M009

## Description

Establishes the dependency foundation and feature-flag wiring for the `telemetry` feature on assay-core and assay-cli. Adds OTel crates as optional workspace dependencies with the correct transport features (http-proto + hyper-client) to avoid the reqwest version conflict flagged by deny.toml. Creates red-state integration tests that define the OTLP contract before any OTel code is written.

## Steps

1. Add OTel workspace deps to root `Cargo.toml` `[workspace.dependencies]`:
   - `opentelemetry = { version = "0.31", optional = true }`
   - `opentelemetry_sdk = { version = "0.31", optional = true, features = ["rt-tokio"] }`
   - `opentelemetry-otlp = { version = "0.31", optional = true, default-features = false, features = ["http-proto", "hyper-client"] }`
   - `tracing-opentelemetry = { version = "0.32", optional = true }`
   - Add `"registry"` to existing `tracing-subscriber` features list (currently `["fmt", "env-filter"]`)
   - Note: `optional = true` at workspace level means the dep is only pulled when a crate explicitly enables it
2. Add `telemetry` feature to `crates/assay-core/Cargo.toml`:
   - `[features]` section gains: `telemetry = ["dep:opentelemetry", "dep:opentelemetry_sdk", "dep:opentelemetry-otlp", "dep:tracing-opentelemetry"]`
   - Add each OTel crate as `opentelemetry = { workspace = true, optional = true }` (repeat for all 4)
3. Add `telemetry` feature to `crates/assay-cli/Cargo.toml`:
   - `[features]` section gains: `telemetry = ["assay-core/telemetry"]`
4. Run `cargo deny check bans` to verify no new version conflicts. If prost or hyper versions conflict, add targeted `skip` entries to `deny.toml` with clear reason comments.
5. Create `crates/assay-core/tests/telemetry_otlp.rs` with red-state integration tests (behind `#[cfg(feature = "telemetry")]`):
   - `test_otel_layer_init_compiles`: call `init_tracing(TracingConfig { otlp_endpoint: Some("http://localhost:4318".into()), ..Default::default() })` — asserts it returns a TracingGuard without panicking (will fail until T02 adds the field + OTel wiring)
   - `test_traceparent_injected_in_subprocess`: create a parent span, call a subprocess that prints TRACEPARENT, assert the env var matches W3C format (will fail until T03 adds injection)
   - Verify tests compile with `--features telemetry` (even though they fail at runtime)

## Must-Haves

- [ ] `telemetry` feature defined on assay-core Cargo.toml enabling 4 optional OTel deps
- [ ] `telemetry` feature defined on assay-cli Cargo.toml enabling `assay-core/telemetry`
- [ ] `tracing-subscriber` workspace features include `registry`
- [ ] `cargo deny check bans` passes (no new violations)
- [ ] `cargo build -p assay-cli` (default, no features) still compiles
- [ ] `crates/assay-core/tests/telemetry_otlp.rs` exists and compiles with `--features telemetry`

## Verification

- `cargo build -p assay-cli` — default build compiles (no OTel pulled in)
- `cargo build -p assay-core --features telemetry` — feature-flagged build compiles
- `cargo deny check bans` — no new violations
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — compiles (tests expected to fail — red state)

## Observability Impact

- Signals added/changed: None (foundation task — no runtime behavior yet)
- How a future agent inspects this: `cargo tree -p assay-core --features telemetry | grep opentelemetry` shows OTel dep tree; without feature, same command returns empty
- Failure state exposed: `cargo deny check bans` catches version conflicts immediately

## Inputs

- `Cargo.toml` — existing workspace deps (tokio, tracing-subscriber features)
- `crates/assay-core/Cargo.toml` — existing `[features]` with `orchestrate`
- `crates/assay-cli/Cargo.toml` — existing deps on assay-core with `orchestrate` feature
- S05-RESEARCH.md — version pinning (opentelemetry 0.31, tracing-opentelemetry 0.32), transport choice (http-proto + hyper-client)
- D143 — rt-tokio with existing runtime, no scoped runtime

## Expected Output

- `Cargo.toml` — 4 new workspace deps + registry in tracing-subscriber features
- `crates/assay-core/Cargo.toml` — `telemetry` feature + 4 optional deps
- `crates/assay-cli/Cargo.toml` — `telemetry` feature
- `crates/assay-core/tests/telemetry_otlp.rs` — red-state test file (2+ tests)
- `deny.toml` — potential skip entries if needed
