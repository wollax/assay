---
id: S03
milestone: M011
status: ready
---

# S03: Health endpoint + final verification — Context

## Goal

Add an unauthenticated `GET /health` endpoint to `smelt serve` and run a full milestone verification pass confirming all M011 success criteria are met.

## Why this Slice

S03 depends on both S01 (decomposition) and S02 (tracing migration + flaky test fix) being complete. The health endpoint is the last feature deliverable of M011. The final verification task ensures every milestone success criterion is proven in one pass before closing the milestone.

## Scope

### In Scope

- Add `GET /health` route to the axum router in `http_api.rs`
- Health endpoint returns `200` with `{"status": "ok"}` JSON body (minimal — no version, uptime, or queue stats)
- Health route bypasses auth middleware even when `[auth]` is configured (D140)
- One-liner documentation of `GET /health` in the existing API section of the README
- Dedicated verification task that runs every M011 success criterion in one pass:
  - `cargo test --workspace` — 0 failures, ≥290 tests
  - `cargo clippy --workspace` — zero warnings
  - `cargo doc --workspace --no-deps` — zero warnings
  - No production source file exceeds 500 lines
  - Zero `eprintln!` calls in `crates/smelt-cli/src/` (S02 will have migrated all, including `main.rs`)
  - `GET /health` returns 200 against a running `smelt serve` instance with auth configured

### Out of Scope

- Readiness/liveness probes with detailed status (just a simple health check)
- Version, uptime, or queue statistics in the health response body
- Metrics or Prometheus endpoint
- Dedicated health endpoint subsection in README (one-liner is sufficient)
- JSON structured log output format or OpenTelemetry

## Constraints

- D140: Health endpoint must be unauthenticated — load balancers and monitoring probe without credentials
- D135: Auth middleware uses `Option<ResolvedAuth>` as state, always applied via `from_fn_with_state` — health route must bypass this (split router groups or path check in middleware)
- D127/D070: `deny(missing_docs)` enforced on both crates — all new public items need docs
- S01 and S02 must be complete before S03 starts (dependency in roadmap)

## Integration Points

### Consumes

- `crates/smelt-cli/src/serve/http_api.rs` — existing `build_router()` function where health route is added
- `crates/smelt-cli/src/serve/http_api.rs` — existing `auth_middleware()` that health must bypass
- S01 output: decomposed codebase (all files under 500L)
- S02 output: all `eprintln!` migrated to tracing, flaky test fixed

### Produces

- `GET /health` route in `http_api.rs` returning `{"status": "ok"}` with 200 status
- Health route bypassing auth middleware
- One-liner README addition documenting the health endpoint
- Full milestone verification report confirming all success criteria pass

## Open Questions

- None — all behavioral decisions settled during discussion.
