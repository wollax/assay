---
id: T02
parent: S04
milestone: M009
provides:
  - TracesCommand enum (List, Show) with handle() dispatcher in crates/assay-cli/src/commands/traces.rs
  - assay traces list — scans .assay/traces/*.json, prints table of ID/Timestamp/Root Span/Spans
  - assay traces show <id> — loads trace JSON, renders indented span tree with durations
key_files:
  - crates/assay-cli/src/commands/traces.rs
  - crates/assay-cli/src/commands/mod.rs
  - crates/assay-cli/src/main.rs
key_decisions:
  - "Traces subcommand falls through to TracingConfig::default() (traces_dir: None) — no self-tracing loop"
  - "print_span_tree builds HashMap<Option<u64>, Vec<&SpanData>> for O(n) tree construction matching T01's parent_id field"
patterns_established:
  - "TracesCommand follows the same Subcommand enum + handle() dispatcher pattern as HistoryCommand"
  - "span tree rendered by recursive render_span() with depth*2 indent — reusable pattern for any tree over SpanData"
observability_surfaces:
  - "tracing::warn! on unreadable trace file (id, error) during list scan"
  - "tracing::error! with id and path on missing trace file in show"
  - "tracing::error! with id and error on malformed JSON in show"
duration: ~1h
verification_result: passed
completed_at: 2026-03-25
blocker_discovered: false
---

# T02: CLI `assay traces list` and `assay traces show` subcommands

**Added `assay traces list` and `assay traces show <id>` subcommands that scan `.assay/traces/` and render an indented span tree from JSON trace files written by T01's JsonFileLayer.**

## What Happened

Created `crates/assay-cli/src/commands/traces.rs` with:
- `TracesCommand` enum (`List`, `Show { id: String }`) using the same Subcommand pattern as `HistoryCommand`
- `handle_list()`: scans `.assay/traces/*.json`, parses each file, extracts root span name and first span timestamp, prints a table sorted chronologically by filename (ID / Timestamp / Root Span / Spans columns)
- `handle_show(id)`: loads `.assay/traces/<id>.json`, builds a `HashMap<Option<u64>, Vec<&SpanData>>` adjacency map from `parent_id`, renders recursively with 2-space indentation and `duration_ms` per span
- Structured error handling: missing dir → exit 1 with tracing::error!, missing file → exit 1, malformed JSON → exit 1; unreadable files during list scan → tracing::warn! (skipped, not fatal)

Registered `pub(crate) mod traces;` in `commands/mod.rs`, added `Traces { command }` variant to the `Command` enum in `main.rs`, added dispatch arm, and confirmed `tracing_config_for()` returns `TracingConfig::default()` (traces_dir: None) for the Traces path.

## Verification

- `cargo test -p assay-cli traces` — 7 tests passed (list parses files, sorts by filename, empty dir, tree root, parent-child tree, missing file detection, malformed JSON error)
- `cargo build -p assay-cli` — compiles cleanly
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (no errors)

## Diagnostics

- `assay traces list` — run from any project directory; scans `.assay/traces/` and prints the table
- `assay traces show <id>` — run from any project directory; shows indented span tree with timing
- On missing `.assay/traces/` directory: tracing::error! with path, exit code 1
- On missing trace ID: tracing::error! with id and path, exit code 1
- On malformed JSON: tracing::error! with id and error, exit code 1

## Deviations

- `TraceRow` struct defined at module level rather than inside `handle_list()` — required by `print_trace_list()` taking `&[TraceRow]` (Rust scoping rules)

## Known Issues

none

## Files Created/Modified

- `crates/assay-cli/src/commands/traces.rs` — new module: TracesCommand, handle_list, handle_show, print_span_tree, render_span, 7 unit tests
- `crates/assay-cli/src/commands/mod.rs` — added `pub(crate) mod traces;`
- `crates/assay-cli/src/main.rs` — added Traces variant to Command enum, dispatch arm in run(), Traces in tracing_config_for() falls through to default (no self-tracing)
