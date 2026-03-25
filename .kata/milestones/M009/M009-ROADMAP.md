# M009: Observability

**Vision:** Add OpenTelemetry distributed tracing to Assay's orchestration, pipeline, and merge paths. Replace unstructured `eprintln!` with leveled `tracing::*` events. Export traces to JSON files for local inspection and OTLP for Jaeger/Grafana Tempo. Propagate trace context across subprocess boundaries. After M009, every multi-agent orchestration run produces a complete, inspectable trace tree.

## Success Criteria

- `RUST_LOG=debug assay run manifest.toml` produces structured, leveled log output to stderr (not bare eprintln)
- Zero `eprintln!` calls remain in assay-core, assay-cli, assay-mcp, or assay-tui (replaced with tracing macros)
- A single-agent pipeline run produces spans for each stage (spec_load, worktree_create, agent_launch, gate_eval, merge_propose) visible in trace output
- A 3-session DAG orchestration run produces nested spans: orchestration root → per-session → pipeline stages → gate evals, all with timing
- `.assay/traces/` contains JSON trace files after an instrumented run; `assay traces list` and `assay traces show <id>` display them
- With `--features telemetry` and Jaeger running, traces appear in Jaeger UI with correct parent-child span relationships
- `cargo build` (default features) does not pull in tokio or opentelemetry crates
- `TRACEPARENT` env var is set on child processes spawned by the pipeline
- `just ready` passes with all new tests green

## Key Risks / Unknowns

- **Sync core + async OTLP exporter** — tokio must be scoped to the exporter only, not leak into business logic
- **Feature flag dep isolation** — `telemetry` feature must cleanly gate all OTel/tokio deps; `cargo build` without it must not pull them
- **Thread-crossing spans in DAG executor** — `std::thread::scope` workers must correctly parent spans to the orchestration root
- **eprintln migration volume** — ~250 calls; mechanical but must preserve stderr behavior for CLI users

## Proof Strategy

- Sync+async isolation risk → retire in S05 by building the real OTLP exporter with scoped tokio and proving default build has no tokio dep
- Thread-crossing spans risk → retire in S03 by instrumenting the DAG executor with cross-thread span parenting and verifying in tests
- Feature flag isolation → retire in S05 by proving `cargo build` (no features) excludes OTel/tokio from the dep tree

## Verification Classes

- Contract verification: unit tests (subscriber setup, span creation, JSON file writing, CLI trace commands), integration tests (pipeline spans, orchestration span nesting)
- Integration verification: real Jaeger instance receives traces from OTLP exporter (UAT)
- Operational verification: `just ready` passes; `cargo build` (default) has no new deps
- UAT / human verification: Jaeger UI shows correct trace tree for a multi-session orchestration run

## Milestone Definition of Done

This milestone is complete only when all are true:

- All slice checkboxes are `[x]` in this roadmap
- Zero `eprintln!` in production code (assay-core, assay-cli, assay-mcp, assay-tui)
- Pipeline stages produce named spans with timing
- Orchestration produces nested span trees (root → sessions → stages)
- JSON trace files appear under `.assay/traces/` and are inspectable via CLI
- OTLP export works behind `--features telemetry` and sends to Jaeger
- `TRACEPARENT` env var set on subprocess spawns
- Default `cargo build` does not pull tokio or OTel deps
- `just ready` passes with zero warnings
- All success criteria re-checked against running code

## Requirement Coverage

- Covers: R027 (OTel instrumentation — all slices), R060 (tracing foundation — S01), R061 (pipeline spans — S02), R062 (orchestration spans — S03), R063 (JSON export — S04), R064 (OTLP export — S05), R065 (context propagation — S05)
- Partially covers: none
- Leaves for later: R066 (TUI trace viewer), R067 (OTel metrics)
- Orphan risks: none — all 7 active requirements mapped to slices

## Slices

- [x] **S01: Structured tracing foundation and eprintln migration** `risk:medium` `depends:[]`
  > After this: all crates emit structured `tracing::*` events instead of `eprintln!`. `RUST_LOG=debug assay gate run spec` produces leveled, structured output to stderr. Proven by grep confirming zero eprintln in production code + integration test exercising log output.

- [x] **S02: Pipeline span instrumentation** `risk:medium` `depends:[S01]`
  > After this: a single-agent pipeline run produces named spans for each stage (spec_load, worktree_create, agent_launch, gate_eval, merge_propose) with timing and spec slug. Proven by integration test asserting span names in captured subscriber output.

- [x] **S03: Orchestration span instrumentation** `risk:high` `depends:[S01, S02]`
  > After this: a DAG/Mesh/Gossip orchestration run produces a nested span tree: orchestration root → per-session → pipeline stages. Merge runner phases and conflict resolution are instrumented. Proven by integration test with mock runners verifying span parent-child relationships across threads.

- [ ] **S04: JSON file trace export and CLI** `risk:low` `depends:[S01]`
  > After this: instrumented runs write JSON trace files to `.assay/traces/`. `assay traces list` shows recent traces; `assay traces show <id>` renders a span tree with timing. Proven by integration tests with synthetic trace data + CLI output assertions.

- [ ] **S05: OTLP export and trace context propagation** `risk:high` `depends:[S01, S02, S03]`
  > After this: with `--features telemetry`, spans export to an OTLP collector. Scoped tokio runtime for async export. `TRACEPARENT` env var injected into subprocess spawns. `cargo build` (default) has no tokio/OTel deps. Proven by feature-flag dep check + integration test verifying TRACEPARENT propagation.

## Boundary Map

### S01 → S02, S03, S04, S05

Produces:
- `tracing-subscriber` layered initialization: `init_tracing(config) -> TracingGuard` free function in a new `assay-core::telemetry` module
- `TracingGuard` type that holds subscriber state and flushes on drop
- All crates using `tracing::*` macros instead of `eprintln!`
- `RUST_LOG` env filter support via `tracing_subscriber::EnvFilter`

Consumes:
- nothing (foundation slice)

### S02 → S03

Produces:
- `#[instrument]` spans on `run_session()`, `setup_session()`, `execute_session()`, and individual pipeline stage functions
- Span fields: `stage`, `spec_slug`, `session_name`
- Pattern for child spans in pipeline sub-functions

Consumes from S01:
- `tracing` macros available throughout assay-core

### S03 (standalone after S01+S02)

Produces:
- Orchestration root span wrapping `run_orchestrated()`, `run_mesh()`, `run_gossip()`
- Per-session child spans in DAG/Mesh/Gossip executors
- Merge runner spans: `merge_completed_sessions` root, per-session merge, conflict resolution
- Cross-thread span parenting in `std::thread::scope` workers

Consumes from S02:
- Pipeline spans (nested inside per-session orchestration spans)

### S04 (standalone after S01)

Produces:
- JSON file trace layer writing to `.assay/traces/<trace_id>.json`
- `assay traces list` CLI subcommand
- `assay traces show <id>` CLI subcommand with span tree rendering
- Trace file pruning configuration

Consumes from S01:
- `init_tracing()` accepts a JSON file layer configuration

### S05 (standalone after S01+S02+S03)

Produces:
- `telemetry` Cargo feature on assay-core and assay-cli
- `opentelemetry-otlp` exporter with scoped tokio runtime
- `OTEL_EXPORTER_OTLP_ENDPOINT` env var + `config.toml` endpoint configuration
- `TRACEPARENT` env var injection in `launch_agent()` and `launch_agent_streaming()`
- `init_tracing()` conditionally adds OTel layer when feature enabled and endpoint configured

Consumes from S01:
- Layered subscriber architecture
Consumes from S02+S03:
- Pipeline and orchestration spans (exported to OTLP)
