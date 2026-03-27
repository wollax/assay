# M013: Tech Debt & Deferred Features

**Vision:** Clear four categories of known-deferred work that accumulated across M001–M012: fix GitHubBackend silent construction failures (Q001–Q004), surface trace data in the TUI (R066), add OTel metrics alongside the existing tracing spans (R067), and make the guided wizard produce immediately runnable gate criteria (R082/D076). No new architectural surfaces — all work targets existing, well-understood subsystems.

## Success Criteria

- `GitHubBackend::new` with a malformed repo emits a `tracing::warn!` at construction time; issue number `0` is rejected in `read_issue_number` with a clear error
- TUI `t` key from Dashboard opens a trace viewer listing recent `.assay/traces/*.json` files; span tree visible with Enter/arrow navigation; Esc closes
- `cargo build --features telemetry` produces a binary that increments OTel counters during `gate run` and records gate-eval latency histograms; `just ready` green
- `assay plan` wizard prompts for an optional command per criterion; generated gates.toml has `cmd` field when provided; `gate run` succeeds immediately on wizard output without manual editing
- Q003: `GhRunner` error helper extracted (all three methods use it); Q004: factory.rs doc cleaned
- All 1529+ existing tests pass; new contract tests cover each fix

## Key Risks / Unknowns

- **OTel metrics SDK API** — `opentelemetry_sdk` 0.31 changed the metrics API. Need to verify `MeterProvider`, `Counter`, and `Histogram` creation before designing S03.
- **Trace file coupling** — trace viewer reads `JsonFileLayer` output. Format must match exactly — any drift causes silent parse failures.
- **Wizard flow UX** — adding an optional `cmd` prompt per criterion extends an already multi-step flow; must stay non-blocking (skippable).

## Proof Strategy

- OTel metrics API → retire in S03 by building `init_metrics()` against actual SDK and running a smoke test that confirms counter increments
- Trace file coupling → retire in S02 by reading real `.assay/traces/` files written by the existing `JsonFileLayer` and rendering them without errors

## Verification Classes

- Contract verification: unit tests for Q001–Q004 (construction validation, issue-0 rejection, error helper, doc cleanup); wizard criteria cmd round-trip test; trace viewer renders from real trace JSON
- Integration verification: `gate run` increments real OTel counters (with feature flag); trace viewer integration test reads a real trace file written by `JsonFileLayer`
- Operational verification: none (no new daemons)
- UAT / human verification: real OTLP collector (Jaeger/Tempo) receiving metrics; wizard end-to-end with TTY

## Milestone Definition of Done

This milestone is complete only when all are true:

- `GitHubBackend::new` warns on malformed repo; `read_issue_number` rejects `0`; GhRunner error helper extracted; factory.rs doc cleaned
- TUI trace viewer screen exists, renders span tree from real trace JSON, accessible via `t` key
- OTel counters and histograms defined and increment during `gate run` under `telemetry` feature; `MeterProvider` stored in `TracingGuard` for clean shutdown
- `create_spec_from_params` accepts `cmd: Option<String>` per criterion; CLI wizard prompts for cmd (skippable); generated gates.toml has `cmd` field when set
- `just ready` green with 1529+ tests; new contract/integration tests for each item

## Requirement Coverage

- Covers: R066, R067, R081, R082
- Partially covers: none
- Leaves for later: LinearBackend checkpoint support, per-session adapter selection (D040), worktree_cleanup_all MCP tool
- Orphan risks: none

## Slices

- [ ] **S01: GitHubBackend correctness fixes (Q001–Q004)** `risk:low` `depends:[]`
  > After this: `GitHubBackend` warns on malformed repo, rejects issue `0`, has extracted GhRunner error helper, and factory.rs doc is clean — all proven by unit tests.

- [ ] **S02: TUI trace viewer** `risk:medium` `depends:[]`
  > After this: TUI `t` key opens a trace list screen; span tree visible from real `.assay/traces/` JSON; Esc closes — proven by integration test reading real trace output.

- [ ] **S03: OTel metrics** `risk:medium` `depends:[]`
  > After this: `gate run --features telemetry` increments session/gate/merge counters and records latency histograms; `MeterProvider` cleanly shuts down; `just ready` green.

- [ ] **S04: Wizard runnable criteria** `risk:low` `depends:[]`
  > After this: `assay plan` wizard collects an optional `cmd` per criterion; generated gates.toml has `cmd` when provided; `gate run` succeeds immediately on wizard output without manual editing.

## Boundary Map

### S01 → (independent)

Produces:
- `GitHubBackend::new` — warns via `tracing::warn!` when `repo` is empty or missing `/`
- `read_issue_number` — returns `Err` when parsed number is `0`
- `GhRunner::gh_error(operation, status, stderr) -> AssayError` — shared error helper
- Factory.rs doc comment — clean of milestone identifiers

Consumes: nothing (leaf fixes)

### S02 → (independent)

Produces:
- `Screen::TraceViewer { traces: Vec<TraceEntry>, selected: usize }` variant in `assay-tui::app`
- `TraceEntry { id, root_span_name, span_count, duration_ms, timestamp }` — parsed from `JsonFileLayer` output
- `draw_trace_viewer(frame, area, traces, selected)` render function
- `t` key handler in `App::handle_event` (from Dashboard)
- Integration test: reads a real `JsonFileLayer`-written trace file and renders without error

Consumes: `crates/assay-core/src/telemetry/trace_export.rs` — existing `JsonFileLayer` output format

### S03 → (independent)

Produces:
- `init_metrics(config: &TracingConfig) -> Option<SdkMeterProvider>` in `assay_core::telemetry`
- `TracingGuard.meter_provider: Option<SdkMeterProvider>` — drop triggers shutdown
- Global counters: `sessions_launched`, `gates_evaluated`, `merges_attempted`
- Global histograms: `gate_eval_latency_ms`, `agent_run_duration_ms`
- All behind `#[cfg(feature = "telemetry")]`

Consumes: existing `init_tracing()` / `TracingGuard` in `assay_core::telemetry`; D144 http-proto transport

### S04 → (independent)

Produces:
- `WizardCriterionInput { description: String, cmd: Option<String> }` (or extends existing)
- `create_spec_from_params(criteria: Vec<WizardCriterionInput>)` — writes `cmd` field when `Some`
- CLI wizard: after each criterion description prompt, optional cmd prompt (Enter skips)
- MCP `spec_create` tool: `criteria` param accepts objects with optional `cmd` field
- Test: wizard round-trip produces gates.toml with `cmd` field set; `gate run` succeeds on output

Consumes: `crates/assay-core/src/wizard.rs`, `crates/assay-cli/src/commands/spec.rs`
