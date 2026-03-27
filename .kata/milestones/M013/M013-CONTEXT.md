# M013: Tech Debt & Deferred Features — Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

## Project Description

Assay is a spec-driven development platform for AI-augmented workflows, built in Rust. M001–M012 are complete. The codebase is ~26K lines across 7 crates with 1529 tests passing. The platform has a full TUI, CLI, MCP server, multi-agent orchestration, pluggable state backends, and complete OpenTelemetry tracing infrastructure.

M013 clears four categories of known-but-deferred work that accumulated across M001–M012:

1. **Q001–Q004**: GitHubBackend construction-time validation and factory doc cleanup (PR #193 review backlog)
2. **R066**: TUI trace viewer — traces are written to `.assay/traces/` by M009's `JsonFileLayer`, but the TUI has no screen to inspect them
3. **R067**: OTel metrics — counters and histograms alongside the existing tracing spans
4. **D076**: Wizard `cmd` collection — `assay plan` generates gates.toml with description-only criteria that can't run until manually edited

## Why This Milestone

These items have well-understood scope and real foundations in place. Doing them now:
- Q001–Q004 are bite-sized correctness improvements that prevent silent runtime failures
- R066 closes the trace inspection loop: M009 built the data, M013 surfaces it
- R067 adds the aggregate-trend layer (metrics) that tracing alone can't provide
- D076 makes the guided wizard actually useful — currently its output requires manual surgery before gates can run

## User-Visible Outcome

### When this milestone is complete, the user can:
- Configure a GitHub backend and get a clear error at construction time if `repo` is malformed (instead of a confusing `gh` error at runtime)
- Press `t` in the TUI to open a trace viewer showing recent orchestration runs with span tree and timing
- Configure OTLP export and see counters (sessions launched, gates evaluated) and histograms (gate eval latency) in Jaeger/Grafana
- Run `assay plan`, answer the wizard questions, and get a gates.toml with runnable `cmd` fields — no manual editing required

### Entry point / environment
- Entry point: `assay plan` (wizard), `assay-tui` (trace viewer), `assay run`/`assay gate run` (metrics), `GitHubBackend::new` (validation)
- Environment: Rust codebase; OTel metrics with real collector is UAT only
- Live dependencies: `gh` CLI for GitHub backend; OTLP collector (Jaeger/Tempo) for metrics UAT

## Completion Class

- Contract complete means: Q001–Q004 verified by unit tests; trace viewer screen renders from real `.assay/traces/` JSON files; OTel metrics init without errors; wizard produces runnable criteria with tests
- Integration complete means: `just ready` green with all tests passing; trace viewer exercises real `JsonFileLayer` output; metrics increment on real gate runs
- Operational complete means: none (no new daemons or lifecycle concerns)

## Final Integrated Acceptance

To call this milestone complete:
- `GitHubBackend::new("no-slash")` returns an error (or emits a clear warning); issue `0` rejected by `read_issue_number`; GhRunner error helper extracted; factory.rs doc cleaned
- TUI `t` key opens trace viewer; span tree renders from a real `.assay/traces/*.json` file; Esc closes it
- OTel counters increment during a `gate run`; histograms record latency; `just ready` green
- `assay plan` wizard collects `cmd` per criterion; generated gates.toml has `cmd` field; gate can run immediately without manual edits

## Risks and Unknowns

- **OTel metrics SDK API surface** — `opentelemetry_sdk` 0.31 changed the metrics API significantly from 0.30. Need to verify the current API before designing S03.
- **TUI trace file coupling** — the trace viewer reads `.assay/traces/*.json` files written by `JsonFileLayer`. The file format is the coupling surface — any deviation between writer and reader will cause silent parse failures. Must validate against real written files.
- **Wizard `cmd` UX** — collecting a command per criterion in the terminal wizard adds steps to an already multi-step flow. Need to decide: optional with a default (`cargo check`?), mandatory, or skippable.

## Existing Codebase / Prior Art

- `crates/assay-backends/src/github.rs` — `GitHubBackend::new`, `GhRunner`, `read_issue_number` (Q001–Q003 targets)
- `crates/assay-backends/src/factory.rs` — `backend_from_config` doc comment (Q004 target)
- `crates/assay-core/src/telemetry.rs` — `init_tracing()`, `TracingConfig`, `TracingGuard` (metrics layer attaches here)
- `crates/assay-tui/src/app.rs` — `App`, `Screen` enum (trace viewer screen added here)
- `crates/assay-core/src/telemetry/trace_export.rs` — `JsonFileLayer`, trace file format (reader for trace viewer)
- `crates/assay-core/src/wizard.rs` (or `assay-core::wizard` module) — `create_from_inputs` (wizard `cmd` fix)
- `crates/assay-cli/src/commands/spec.rs` — `plan` command, dialoguer-based wizard (cmd collection added here)
- `crates/assay-mcp/src/server.rs` — `milestone_create`/`spec_create` tools (cmd field added to MCP params)
- D135/D136: tracing-test pattern for span assertions — same dev-dep applies to metrics
- D144: http-proto + hyper-client transport for OTLP (metrics uses the same transport)
- D147: `SdkTracerProvider` stored in `TracingGuard` — metrics provider stored analogously

> See `.kata/DECISIONS.md` for all architectural decisions.

## Relevant Requirements

- R066 — TUI trace viewer (deferred since M009)
- R067 — OTel metrics (deferred since M009)
- R081 (new) — GitHubBackend construction validation (Q001–Q002)
- R082 (new) — Wizard runnable criteria (D076)

## Scope

### In Scope

- Q001: `GitHubBackend::new` validates `owner/repo` format; emits `tracing::warn!` when malformed
- Q002: `read_issue_number` rejects issue number `0`
- Q003: `GhRunner` error helper extracted (reduces duplication in create_issue, create_comment, get_issue_json)
- Q004: `factory.rs` doc comment cleaned of milestone identifiers
- R066: TUI trace viewer screen — `t` key from Dashboard opens a list of recent traces; Enter/arrow keys navigate into a span tree; Esc closes
- R067: OTel metrics — counters (sessions_launched, gates_evaluated, merges_attempted) and histograms (gate_eval_latency_ms, agent_run_duration_ms); feature-flagged under existing `telemetry` feature; `MeterProvider` stored in `TracingGuard` alongside `SdkTracerProvider`
- R082: Wizard `cmd` collection — `create_from_inputs` / `create_spec_from_params` gain an optional `cmd: Option<String>` per criterion; CLI wizard prompts for a command after each criterion description (skippable); generated gates.toml has `cmd` field when provided

### Out of Scope / Non-Goals

- Per-session adapter selection (D040) — orthogonal, larger scope
- `worktree_cleanup_all` MCP tool — separate concern
- LinearBackend checkpoint support — separate concern
- Real OTLP collector validation (UAT only)
- Metrics dashboards or alerting configuration

## Technical Constraints

- D001: Zero-trait convention. No new trait objects.
- D005: MCP tools are additive only — no signature changes to existing tools.
- D144: OTLP transport is http-proto + hyper-client (avoid reqwest version conflict).
- D147 analogy: `MeterProvider` stored in `TracingGuard` for shutdown.
- Feature gate: metrics live under `#[cfg(feature = "telemetry")]` — default build must have zero OTel deps added.
- `just ready` must stay green throughout.

## Open Questions

- **Wizard `cmd` optional vs mandatory?** — Decision: optional. If the user leaves it blank, no `cmd` field is written (same as today). If provided, written to gates.toml. This keeps the wizard non-blocking for users who don't know the command yet.
- **Trace viewer list scope?** — Decision: show the 20 most recent traces (same cap as `assay traces list`), sorted by timestamp descending. Each entry shows: run timestamp, root span name, total span count, and duration.
