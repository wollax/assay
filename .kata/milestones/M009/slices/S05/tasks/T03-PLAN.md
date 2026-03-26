---
estimated_steps: 4
estimated_files: 3
---

# T03: Inject TRACEPARENT into launch_agent and launch_agent_streaming subprocess spawns

**Slice:** S05 — OTLP export and trace context propagation
**Milestone:** M009

## Description

Delivers R065 (trace context propagation). Injects the W3C TRACEPARENT env var into child processes spawned by `launch_agent()` and `launch_agent_streaming()` so agent subprocess traces correlate with the parent orchestration trace. Uses the global OTel propagator to extract span context from the current tracing span and serialize it as a TRACEPARENT header value. Feature-gated: when `telemetry` is not compiled in, no code change affects subprocess spawning.

## Steps

1. In `launch_agent()` (crates/assay-core/src/pipeline.rs), after `Command::new("claude")` and before `.spawn()`, add a `#[cfg(feature = "telemetry")]` block:
   - Get current span: `let span = tracing::Span::current();`
   - Check `!span.is_disabled()` — guard against no active span
   - Use `opentelemetry::global::get_text_map_propagator(|propagator| { ... })` to inject context into a `HashMap<String, String>`
   - Extract `TRACEPARENT` value from the map
   - Call `.env("TRACEPARENT", &traceparent_value)` on the Command builder
   - When span is disabled, emit `tracing::debug!("launch_agent: no active span; TRACEPARENT not injected")`
2. Apply the identical pattern to `launch_agent_streaming()` — same cfg block, same propagator injection, same debug log for disabled span. The Command construction is inside the spawned thread, so the span context must be captured before spawning the thread (capture `Span::current()` in the outer scope, clone into thread).
3. Write the integration test in `telemetry_otlp.rs` — `test_traceparent_injected_in_subprocess`:
   - Set up OTel provider with TraceContextPropagator (reuse the init pattern from T02)
   - Create a parent `info_span!("test_parent")` and enter it
   - Build a Command that runs `printenv TRACEPARENT` (or `env | grep TRACEPARENT`)
   - Apply the same TRACEPARENT injection logic to this test Command
   - Capture stdout and assert the output matches W3C format: `00-{32 hex chars}-{16 hex chars}-{2 hex chars}`
4. Verify the no-feature path: `cargo test -p assay-core --lib -- pipeline` (without `--features telemetry`) still passes — no cfg-gated code affects the default build.

## Must-Haves

- [ ] `launch_agent()` injects TRACEPARENT env var when feature=telemetry and active span exists
- [ ] `launch_agent_streaming()` injects TRACEPARENT env var when feature=telemetry and active span exists
- [ ] No-span guard: debug log emitted when no active span (not silent, not error)
- [ ] Integration test proves TRACEPARENT appears in subprocess output with valid W3C format
- [ ] Default build (no features) pipeline tests unaffected

## Verification

- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — TRACEPARENT injection test passes
- `cargo test -p assay-core --lib -- pipeline` — all existing pipeline tests pass (no feature, no regressions)
- `cargo clippy -p assay-core --features telemetry -- -D warnings` — clean

## Observability Impact

- Signals added/changed: `tracing::debug!("launch_agent: no active span; TRACEPARENT not injected")` when called outside any span; TRACEPARENT value in child process environment
- How a future agent inspects this: child process can read `$TRACEPARENT` to correlate its traces; debug-level log reveals when injection was skipped
- Failure state exposed: Missing TRACEPARENT in child → check debug logs for the "no active span" message; malformed TRACEPARENT → propagator returned empty map (global propagator not set — check init order)

## Inputs

- `crates/assay-core/src/pipeline.rs` — launch_agent() at line 222, launch_agent_streaming() at line 337
- T02 output — OTel provider init in telemetry.rs, TraceContextPropagator set globally
- S05-RESEARCH.md — propagator injection pattern, span disabled guard, W3C format `00-traceid-spanid-flags`
- D130 — TRACEPARENT env var for subprocess context propagation

## Expected Output

- `crates/assay-core/src/pipeline.rs` — cfg-gated TRACEPARENT injection in both launch functions
- `crates/assay-core/tests/telemetry_otlp.rs` — TRACEPARENT assertion test added and passing
