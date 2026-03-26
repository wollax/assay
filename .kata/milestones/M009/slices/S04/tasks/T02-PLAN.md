---
estimated_steps: 5
estimated_files: 4
---

# T02: CLI `assay traces list` and `assay traces show` subcommands

**Slice:** S04 — JSON file trace export and CLI
**Milestone:** M009

## Description

Create the CLI subcommands for inspecting trace files. `assay traces list` scans `.assay/traces/` and prints a table of trace files (run ID, timestamp, root span name, span count). `assay traces show <id>` loads a trace JSON file and renders an indented span tree with durations. Register the `Traces` command in the CLI dispatch and ensure it does NOT enable the JSON file trace layer.

## Steps

1. Create `crates/assay-cli/src/commands/traces.rs` with `TracesCommand` enum (List, Show { id: String }). Implement `handle()` dispatch function following the pattern from `history.rs`.
2. Implement `handle_list()`: scan `.assay/traces/*.json`, for each file parse the JSON array, extract root span (parent_id is null) name and first span's start_time, format as a table with columns: ID (filename without .json), Timestamp, Root Span, Spans. Sort by filename (chronological due to timestamp prefix).
3. Implement `handle_show(id)`: load `.assay/traces/<id>.json`, build a tree from parent_id references (HashMap<Option<u64>, Vec<&SpanRecord>>), render recursively with indentation (2 spaces per depth level), showing `name duration_ms` per span. Handle missing file with a clear error message.
4. Register `Traces` variant in `Command` enum in `main.rs`. Add dispatch arm. In `tracing_config_for()`, ensure `Traces` subcommand gets `traces_dir: None` (no self-tracing). Add `pub(crate) mod traces;` to `commands/mod.rs`.
5. Add unit/integration tests in `traces.rs` or a test module: test `handle_list` with synthetic JSON files in a tempdir, test `handle_show` with a known trace structure asserting indented output, test missing-file error handling.

## Must-Haves

- [ ] `assay traces list` scans `.assay/traces/` and prints a table of trace files
- [ ] `assay traces show <id>` renders an indented span tree with timing
- [ ] Missing trace ID produces a clear error (exit code 1, not panic)
- [ ] `Traces` subcommand does NOT enable the JSON file trace layer
- [ ] Tests prove list output and show tree rendering

## Verification

- `cargo test -p assay-cli traces` — tests pass
- `cargo build -p assay-cli` — compiles cleanly
- `cargo clippy --workspace --all-targets -- -D warnings` — clean

## Observability Impact

- Signals added/changed: None — the CLI is a reader, not a writer
- How a future agent inspects this: run `assay traces list` to see available traces; run `assay traces show <id>` to inspect a specific trace
- Failure state exposed: missing .assay/ directory, missing trace file, malformed JSON — all produce structured error messages to stderr

## Inputs

- `crates/assay-core/src/telemetry.rs` — `SpanData` type definition (for JSON deserialization)
- `crates/assay-cli/src/commands/history.rs` — pattern for CLI subcommand structure
- `crates/assay-cli/src/main.rs` — `Command` enum and `tracing_config_for()`

## Expected Output

- `crates/assay-cli/src/commands/traces.rs` — new module with list/show handlers and tests
- `crates/assay-cli/src/commands/mod.rs` — `pub(crate) mod traces;` added
- `crates/assay-cli/src/main.rs` — `Traces` variant in Command, dispatch arm, tracing_config_for update
