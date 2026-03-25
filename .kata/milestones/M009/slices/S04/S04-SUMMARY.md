---
id: S04
parent: M009
milestone: M009
provides:
  - JsonFileLayer custom tracing Layer writing JSON trace files per root span to a configured directory
  - SpanData serializable struct with name, target, level, span_id, parent_id, start_time, end_time, duration_ms, fields
  - TracingConfig.traces_dir field enabling opt-in trace file export
  - Atomic trace file writes via NamedTempFile+persist pattern with file pruning (max 50 files)
  - assay traces list CLI subcommand (table of ID/Timestamp/Root Span/Spans)
  - assay traces show <id> CLI subcommand (indented span tree with timing)
  - traces_dir wired into tracing_config_for() for Run/Gate/Context subcommands
  - End-to-end integration test: write → read → render round-trip proven
requires:
  - slice: S01
    provides: init_tracing()/TracingConfig layered subscriber architecture and tracing macros throughout codebase
affects:
  - S05
key_files:
  - crates/assay-core/src/telemetry.rs
  - crates/assay-core/tests/trace_export.rs
  - crates/assay-cli/src/commands/traces.rs
  - crates/assay-cli/src/commands/mod.rs
  - crates/assay-cli/src/main.rs
key_decisions:
  - "Mutex<HashMap<u64, SpanData>> for thread-safe span storage — contention negligible at this scale"
  - "Root span detection via parent_id.is_none() — each root closure flushes its entire span tree to one file"
  - "SpanData derives both Serialize and Deserialize for round-trip JSON test assertions"
  - "Added registry feature to tracing-subscriber explicitly (was transitively enabled)"
  - "Traces subcommand uses TracingConfig::default() (traces_dir: None) — prevents self-tracing loop"
  - "print_span_tree builds HashMap<Option<u64>, Vec<&SpanData>> for O(n) tree construction matching parent_id field"
  - "TracesCommand follows same Subcommand enum + handle() dispatcher pattern as HistoryCommand"
patterns_established:
  - "JsonFileLayer: collect spans in-memory, flush on root span close, prune old files — reusable trace export pattern"
  - "with_json_layer() test helper using tracing::subscriber::with_default for isolated subscriber tests"
  - "span tree rendered by recursive render_span() with depth*2 indent — reusable for any tree over SpanData"
observability_surfaces:
  - "tracing::debug! on each trace file write (path, span_count)"
  - "tracing::warn! on write/persist/prune failures with path and error context"
  - "tracing::warn! on unreadable trace file during list scan (skipped, not fatal)"
  - "tracing::error! with id and path on missing trace file in show"
  - "tracing::error! with id and error on malformed JSON in show"
  - ".assay/traces/ directory contents — each JSON file is a self-contained trace"
  - "assay traces list — tabular overview of all traces"
  - "assay traces show <id> — indented span tree for a specific trace"
drill_down_paths:
  - .kata/milestones/M009/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M009/slices/S04/tasks/T02-SUMMARY.md
duration: ~2h (T01: 15min, T02: ~1h, T03: ~30min)
verification_result: passed
completed_at: 2026-03-25
---

# S04: JSON file trace export and CLI

**JsonFileLayer captures span lifecycle events and writes structured JSON trace files to `.assay/traces/`; `assay traces list` and `assay traces show <id>` provide zero-dependency trace inspection from the CLI.**

## What Happened

**T01** added `traces_dir: Option<PathBuf>` to `TracingConfig` and implemented `JsonFileLayer` as a `tracing_subscriber::Layer<S>` (where S: `Subscriber + for<'a> LookupSpan<'a>`). The layer stores span state in a `Mutex<HashMap<u64, SpanData>>`: `on_new_span` captures name/target/level/parent/fields/start_time; `on_record` merges additional fields; `on_close` computes duration, detects root spans (parent_id is None), collects the full span tree, writes atomically via `NamedTempFile+persist`, and prunes old files keeping at most 50. The layer is wired into `init_tracing()` via `.with()` — tracing-subscriber natively supports optional layers via `Option<Layer>`. Integration tests prove tree structure (4 nested spans with correct parent-child relationships), pruning (55 → ≤50), and multiple root spans producing separate files.

**T02** added `crates/assay-cli/src/commands/traces.rs` with `TracesCommand` enum (`List`, `Show { id: String }`) following the same pattern as `HistoryCommand`. `handle_list()` scans `.assay/traces/*.json`, extracts root span name and timestamp, prints a 4-column table sorted by filename. `handle_show(id)` loads the JSON file, builds an adjacency map from `parent_id`, and renders a recursive indented tree with duration. Structured errors (missing dir, missing file, malformed JSON) all exit code 1 with `tracing::error!`. Registered in `commands/mod.rs` and `main.rs` with `tracing_config_for()` returning `TracingConfig::default()` (traces_dir: None) for the Traces path to prevent self-tracing. 7 unit tests pass.

**T03** completed the loop: `tracing_config_for()` in `main.rs` was already wired to set `traces_dir: Some(assay_dir.join("traces"))` for Run/Gate/Context subcommands. Added end-to-end round-trip integration test (`trace_export_end_to_end_write_read_render`) proving: real subscriber writes 4-span tree → JSON file read back → parent-child relationships verified → adjacency map reconstructed for CLI rendering. `just ready` passes.

## Verification

- `cargo test -p assay-core trace_export` — 4/4 integration tests pass (tree structure, pruning, multiple roots, end-to-end round-trip)
- `cargo test -p assay-core telemetry` — 4 unit tests + 1 doctest pass
- `cargo test -p assay-cli traces` — 7/7 unit tests pass (list, sort, empty dir, tree root, parent-child, missing file, malformed JSON)
- `cargo build -p assay-cli` — compiles cleanly
- `just ready` — all checks pass (fmt, clippy, test, deny)

## Requirements Advanced

- R063 (JSON file trace export) — fully implemented and validated: JsonFileLayer writes traces, CLI reads and renders them

## Requirements Validated

- R063 — JSON files appear in `.assay/traces/` after instrumented runs; `assay traces list` shows them; `assay traces show <id>` renders span tree with timing. Proven by integration tests with real tracing subscriber (no mocks).

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T01 used `chrono::DateTime::parse_from_rfc3339` for duration calculation instead of storing raw `Instant` — RFC 3339 strings are already captured for serialization; parsing back is reliable for millisecond-level timing and avoids storing non-serializable types.
- T02's `TraceRow` struct defined at module level rather than inside `handle_list()` — required by `print_trace_list()` taking `&[TraceRow]` (Rust scoping rules).
- T03 was initially skipped by auto-mode recovery. Completed in follow-up: `tracing_config_for()` wiring was already present in `main.rs`; only the end-to-end integration test was missing and has been added.

## Known Limitations

- Guard daemon file logging deferred from S01 remains deferred (not part of S04 scope).
- Trace files are written in chronological order but the filename uses a timestamp+hex scheme — no human-readable run label. Future improvement could add a `--run-label` flag.
- `assay traces list` scans all files; for very large trace dirs (>50 after pruning edge cases) this is O(n) but acceptable at current scale.

## Follow-ups

- S05: OTLP export, scoped tokio runtime, TRACEPARENT propagation — consumes the layered architecture from S01 and spans from S02/S03.
- R066 (TUI trace viewer) remains deferred — depends on R063 (now complete) for data source.

## Files Created/Modified

- `crates/assay-core/src/telemetry.rs` — Extended with SpanData, FieldVisitor, JsonFileLayer, generate_trace_id; TracingConfig.traces_dir field; init_tracing wires optional layer
- `crates/assay-core/tests/trace_export.rs` — 4 integration tests: tree structure, pruning, multiple roots, end-to-end round-trip
- `crates/assay-cli/src/commands/traces.rs` — New: TracesCommand, handle_list, handle_show, print_span_tree, render_span, 7 unit tests
- `crates/assay-cli/src/commands/mod.rs` — Added `pub(crate) mod traces;`
- `crates/assay-cli/src/main.rs` — Traces variant in Command enum, dispatch arm, traces_dir wired in tracing_config_for()
- `Cargo.toml` — Added `registry` feature to tracing-subscriber workspace dep

## Forward Intelligence

### What the next slice should know
- The layered subscriber architecture from S01 is proven stable through S04. S05 can add an OTel layer via the same `.with(Option<OtelLayer>)` pattern without any structural changes to `init_tracing()`.
- `TracingConfig` already has `traces_dir` — S05 needs to add `otlp_endpoint: Option<String>` or similar and gate the OTel layer behind the `telemetry` feature flag.
- The `generate_trace_id()` function in `telemetry.rs` produces `<timestamp>T<hex>` strings — these are trace file stems and also what CLI shows as IDs. S05 should use standard W3C `TRACEPARENT` format (not this internal ID) for subprocess propagation.

### What's fragile
- Duration calculation parses RFC 3339 strings back to compute milliseconds — if system clock changes between span open and close (NTP adjustment), duration could be negative. Acceptable at current scale but worth noting for S05.
- Pruning deletes the oldest files by filename sort order (lexicographic on timestamp strings) — this is correct as long as filenames are generated with `generate_trace_id()`.

### Authoritative diagnostics
- `.assay/traces/` directory — self-contained JSON files, each an array of SpanData objects
- `assay traces list` — fastest way to confirm trace files are being written after a run
- `tracing::debug!` events from JsonFileLayer confirm write path and span count per flush

### What assumptions changed
- T03's `tracing_config_for()` wiring was already present in `main.rs` before T03 ran — the prior recovery had partially completed this task. Only the integration test was missing.
