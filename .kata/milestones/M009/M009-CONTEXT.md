# M009: Observability — Context

**Gathered:** 2026-03-24
**Status:** Ready for planning

## Project Description

M009 adds OpenTelemetry distributed tracing to Assay. Spans instrument the pipeline, orchestration, and merge paths. Traces export to JSON files (zero-dependency local inspection) and OTLP (Jaeger/Grafana Tempo). The existing `eprintln!` logging across the workspace is migrated to structured `tracing::*` events. Trace context propagates across subprocess boundaries via `TRACEPARENT` env vars.

## Why This Milestone

Assay has grown to ~24K lines across 6 crates with 3 orchestration modes (DAG, Mesh, Gossip), a 6-stage pipeline, multi-agent merge with conflict resolution, and background TUI polling. When something goes wrong in a multi-agent run — a session stalls, a merge conflicts unexpectedly, a gate eval times out — there's no structured way to see what happened. `eprintln!` output is unleveled, unstructured, and invisible to any collection system. R027 was deferred since M002 waiting for the orchestration surfaces to stabilize. They have.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Run a multi-agent orchestration and inspect the full span tree (session timings, merge phases, gate evals) as JSON files under `.assay/traces/`
- View traces in Jaeger or Grafana Tempo when OTLP export is enabled
- Use `assay traces list` and `assay traces show <id>` to inspect traces from the CLI
- Benefit from leveled, structured log output instead of bare `eprintln!` — with `RUST_LOG` env var controlling verbosity

### Entry point / environment

- Entry point: `assay run`, `assay traces`, orchestration MCP tools
- Environment: local dev + optional Jaeger/Tempo collector
- Live dependencies involved: none required (JSON export works standalone); Jaeger/Tempo optional for OTLP

## Completion Class

- Contract complete means: spans emit on key pipeline/orchestration paths; JSON files written to `.assay/traces/`; OTLP exporter sends to a collector behind a feature flag
- Integration complete means: real multi-session orchestration produces a complete trace tree readable in Jaeger
- Operational complete means: `just ready` passes; default builds (without `telemetry` feature) have no new runtime deps

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- A 3-session DAG orchestration run produces a JSON trace file with nested spans for each session, merge phase, and gate eval
- With `--features telemetry` and a Jaeger instance running, the same run appears as a complete trace in Jaeger UI
- `RUST_LOG=debug assay run manifest.toml` produces structured log output (not eprintln)
- Default `cargo build` does not pull in tokio or opentelemetry deps (feature-gated)

## Risks and Unknowns

- **Sync core + async OTel exporter** — The OTel SDK's OTLP exporter requires an async runtime. Using a scoped tokio runtime only for the exporter is the standard approach but must be proven not to leak into business logic.
- **tracing span overhead in hot paths** — Gate evaluation can involve many command executions. Span creation overhead must be negligible (<1% of eval time).
- **Feature flag dep isolation** — The `telemetry` feature must cleanly gate all OTel/tokio deps. `cargo build` without the feature must not pull them in.
- **Thread-crossing spans in orchestration** — DAG executor uses `std::thread::scope` with bounded concurrency. Spans created in the parent must parent correctly to spans in worker threads. `tracing` handles this natively via `Span::enter()` / `in_scope()`, but must be verified.
- **eprintln migration scope** — ~250 eprintln! calls. Must not change user-visible behavior (CLI output to stderr should be preserved via fmt subscriber).

## Existing Codebase / Prior Art

- `Cargo.toml` workspace — `tracing = "0.1"`, `tracing-subscriber = "0.3"`, `tracing-appender = "0.2"` already in workspace deps
- `crates/assay-core/Cargo.toml` — already depends on `tracing`
- `crates/assay-mcp/Cargo.toml` — already depends on `tracing`; `server.rs` has ~20 `tracing::warn/info` calls
- `crates/assay-core/src/guard/daemon.rs` — uses `tracing::{error, info, warn}` with structured fields
- `crates/assay-cli/src/commands/mcp.rs` — has `tracing_subscriber::EnvFilter` setup for MCP server
- `crates/assay-core/src/pipeline.rs` — pipeline stages (spec load, worktree create, agent launch, gate eval, merge propose) are the primary instrumentation targets
- `crates/assay-core/src/orchestrate/` — executor.rs (DAG), mesh.rs, gossip.rs, conflict_resolver.rs are the orchestration instrumentation targets
- `crates/assay-core/src/merge.rs` — merge execution is the merge instrumentation target

> See `.kata/DECISIONS.md` for all architectural and pattern decisions — it is an append-only register; read it during planning, append to it during execution.

## Relevant Requirements

- R027 — OpenTelemetry instrumentation (activated from deferred, primary driver)
- R060 — Structured tracing foundation (eprintln migration)
- R061 — Pipeline span instrumentation
- R062 — Orchestration span instrumentation
- R063 — JSON file trace export
- R064 — OTLP trace export (feature-flagged)
- R065 — Trace context propagation across subprocesses

## Scope

### In Scope

- Migrate `eprintln!` → `tracing::*` across all crates
- `tracing-subscriber` layered setup: fmt (always) + JSON file (when `.assay/` exists) + OTel (when feature enabled)
- `#[instrument]` spans on pipeline stages and orchestration paths
- JSON trace files under `.assay/traces/`
- `assay traces list` and `assay traces show <id>` CLI commands
- Feature-flagged OTLP export with scoped tokio runtime
- `TRACEPARENT` env var propagation to child processes
- `OTEL_EXPORTER_OTLP_ENDPOINT` + config.toml endpoint configuration

### Out of Scope / Non-Goals

- TUI trace viewer (deferred to R066)
- OTel metrics (deferred to R067)
- Auto-instrumentation of user/agent code
- Cloud/SaaS trace storage
- Modifying the MCP tool signatures (D005)
- Adding `tracing` dep to assay-types (keep it dep-free)

## Technical Constraints

- Zero-trait convention (D001) — tracing layers composed via functions, not trait objects
- Sync core (D007) — tokio scoped only to OTel exporter thread
- `deny_unknown_fields` on Config — new telemetry config fields need `serde(default, skip_serializing_if)` (D092 pattern)
- Default build must not pull tokio/opentelemetry (feature gate)
- CLI stderr behavior preserved after eprintln migration (fmt subscriber writes to stderr)

## Integration Points

- `tracing-subscriber` — subscriber initialization in CLI main(), MCP server, TUI main()
- `tracing-opentelemetry` — bridges tracing spans to OTel SDK
- `opentelemetry-otlp` — OTLP gRPC/HTTP export (feature-gated)
- `opentelemetry-stdout` or custom JSON layer — file export to `.assay/traces/`
- Child processes (agent launch, gh CLI) — `TRACEPARENT` env var injection

## Open Questions

- Exact JSON trace file format — use `tracing-subscriber` JSON layer output vs. OTel JSON exporter format? The former is simpler (one event per line) but not OTel-compatible; the latter requires the OTel SDK even for file export. Decision: defer to S04 planning.
- Trace file rotation/pruning — should old trace files be auto-pruned? Decision: defer to S04, likely use same pattern as history pruning.
