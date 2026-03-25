# S04: JSON file trace export and CLI

**Goal:** Instrumented runs write JSON trace files to `.assay/traces/`. `assay traces list` shows recent traces; `assay traces show <id>` renders a span tree with timing. Integration tests prove the full loop with synthetic data.
**Demo:** After any instrumented pipeline or orchestration run, a JSON file appears in `.assay/traces/`. Running `assay traces list` shows it. Running `assay traces show <id>` renders the span tree with names, durations, and parent-child nesting.

## Must-Haves

- Custom `tracing-subscriber` Layer in `assay_core::telemetry` that captures span lifecycle (`on_new_span`, `on_record`, `on_close`) and writes a structured JSON file per trace to a configured directory
- JSON trace file format: array of span records with `name`, `target`, `level`, `parent_id`, `start_time`, `end_time`, `duration_ms`, `fields`, `events`
- `TracingConfig` extended with `traces_dir: Option<PathBuf>` — when `Some`, the JSON file layer is added to the subscriber stack
- `assay traces list` CLI subcommand listing trace files with timestamp, run ID, and root span name
- `assay traces show <id>` CLI subcommand rendering an indented span tree with timing
- Trace file pruning (max 50 files, configurable) on write
- CLI entry points for `assay traces list` and `assay traces show` do NOT enable the trace file layer (no self-tracing)
- Integration tests with real subscriber capturing prove JSON file output, tree structure, and CLI rendering

## Proof Level

- This slice proves: integration (real tracing subscriber captures real spans → JSON files → CLI reads and renders)
- Real runtime required: no (tests use in-process subscriber with synthetic spans)
- Human/UAT required: no (CLI output verified by integration test assertions)

## Verification

- `cargo test -p assay-core trace_export` — integration tests proving JSON file layer writes correct files
- `cargo test -p assay-cli traces` — CLI subcommand tests proving list/show output
- `cargo test -p assay-core telemetry` — unit tests proving TracingConfig extension and layer composition
- `just ready` — full workspace green

## Observability / Diagnostics

- Runtime signals: `tracing::debug!` events on trace file write (path, span count, duration); `tracing::warn!` on write failure or prune failure
- Inspection surfaces: `.assay/traces/` directory contents; `assay traces list` CLI output; `assay traces show <id>` CLI output
- Failure visibility: trace write failures logged with path and error; pruning failures logged with count and error; CLI shows structured error for missing trace ID
- Redaction constraints: none — trace data contains span names and fields but no secrets

## Integration Closure

- Upstream surfaces consumed: `assay_core::telemetry::init_tracing()` and `TracingConfig` (S01); `tracing_subscriber::Registry` layer composition; `history::generate_run_id()` pattern (not the function — same timestamp+hex pattern)
- New wiring introduced in this slice: `TracingConfig.traces_dir` field plumbed from CLI `tracing_config_for()` to `init_tracing()`; `assay traces` subcommand registered in CLI dispatch
- What remains before the milestone is truly usable end-to-end: S05 (OTLP export + TRACEPARENT propagation)

## Tasks

- [x] **T01: Custom tracing Layer and JSON file writer** `est:45m`
  - Why: Core capability — the Layer captures span lifecycle events and writes structured JSON per trace. This is the foundation that makes trace files exist.
  - Files: `crates/assay-core/src/telemetry.rs`, `crates/assay-core/Cargo.toml`, `crates/assay-core/tests/trace_export.rs`
  - Do: Add `traces_dir: Option<PathBuf>` to `TracingConfig`. Implement `JsonFileLayer` as a `tracing_subscriber::Layer<S>` (where S: `Subscriber + for<'a> LookupSpan<'a>`) with in-memory `Mutex<HashMap<Id, SpanData>>`. `on_new_span` records name/target/level/parent/fields/start_time. `on_record` merges new fields. `on_close` computes duration, removes from map; if root span (no parent), writes all collected spans as JSON array to `<traces_dir>/<run_id>.json` using atomic NamedTempFile+persist pattern, then prunes old files (max 50). Wire into `init_tracing()` via `.with()` when `traces_dir` is Some. Create integration test file that installs subscriber with JsonFileLayer, creates spans, and asserts JSON file contents.
  - Verify: `cargo test -p assay-core trace_export` and `cargo test -p assay-core telemetry`
  - Done when: running a traced function produces a JSON file in the configured traces dir with correct span tree structure

- [ ] **T02: CLI `assay traces list` and `assay traces show` subcommands** `est:30m`
  - Why: User-facing inspection surface — without CLI commands, trace files are opaque JSON blobs
  - Files: `crates/assay-cli/src/commands/traces.rs`, `crates/assay-cli/src/commands/mod.rs`, `crates/assay-cli/src/main.rs`
  - Do: Create `traces.rs` command module with `list` and `show` subcommands. `list` scans `.assay/traces/`, reads each JSON file's root span name and timestamp, prints a table. `show <id>` loads the JSON file, builds a tree from parent_id references, renders indented span names with durations. Register `Traces` variant in main.rs `Command` enum and dispatch. Ensure `tracing_config_for()` returns config with `traces_dir: None` for the `Traces` subcommand.
  - Verify: `cargo test -p assay-cli traces` and `cargo build -p assay-cli`
  - Done when: `assay traces list` prints a table of trace files; `assay traces show <id>` renders an indented span tree with timing

- [ ] **T03: Wire traces_dir into CLI and end-to-end integration test** `est:25m`
  - Why: Closes the loop — CLI commands that run pipelines produce trace files, and the traces CLI reads them. Also wires the traces_dir from CLI entry point to init_tracing.
  - Files: `crates/assay-cli/src/main.rs`, `crates/assay-core/tests/trace_export.rs`
  - Do: Update `tracing_config_for()` to set `traces_dir: Some(assay_dir.join("traces"))` for pipeline-running subcommands (Run, Gate, etc). Add end-to-end integration test that creates spans via a real subscriber, verifies JSON file, then reads it back and asserts tree structure matches. Verify `just ready` passes.
  - Verify: `cargo test -p assay-core trace_export` and `just ready`
  - Done when: `just ready` green; integration test proves write → read → render cycle

## Files Likely Touched

- `crates/assay-core/src/telemetry.rs`
- `crates/assay-core/Cargo.toml`
- `crates/assay-core/tests/trace_export.rs`
- `crates/assay-cli/src/commands/traces.rs`
- `crates/assay-cli/src/commands/mod.rs`
- `crates/assay-cli/src/main.rs`
