---
id: T01
parent: S04
milestone: M009
provides:
  - JsonFileLayer custom tracing Layer that captures span lifecycle and writes JSON trace files
  - SpanData serializable struct for span records with parent-child relationships and timing
  - TracingConfig.traces_dir field for optional trace file export
  - Atomic trace file writes via NamedTempFile+persist pattern
  - File pruning (oldest files removed when count exceeds max_files=50)
key_files:
  - crates/assay-core/src/telemetry.rs
  - crates/assay-core/tests/trace_export.rs
  - Cargo.toml
key_decisions:
  - "Used Mutex<HashMap<u64, SpanData>> for thread-safe span storage — contention negligible at this scale"
  - "Root span detection via parent_id.is_none() heuristic — each root closure flushes its trace tree"
  - "SpanData derives both Serialize and Deserialize for round-trip JSON test assertions"
  - "Added registry feature to tracing-subscriber explicitly (was transitively enabled)"
patterns_established:
  - "JsonFileLayer collects spans in-memory, flushes on root span close, prunes old files — reusable trace export pattern"
  - "with_json_layer() test helper using tracing::subscriber::with_default for isolated subscriber tests"
observability_surfaces:
  - "tracing::debug! on trace file write (path, span_count)"
  - "tracing::warn! on write failure, persist failure, prune failure with path and error context"
  - ".assay/traces/ directory contents — each JSON file is a self-contained trace"
duration: 15min
verification_result: passed
completed_at: 2026-03-24T22:30:00Z
blocker_discovered: false
---

# T01: Custom tracing Layer and JSON file writer

**Implemented JsonFileLayer that captures span lifecycle events and writes structured JSON trace files with atomic writes, parent-child tree structure, timing, and automatic file pruning**

## What Happened

Added `traces_dir: Option<PathBuf>` to `TracingConfig` with backward-compatible `None` default. Implemented `SpanData` struct capturing name, target, level, span_id, parent_id, start_time, end_time, duration_ms, and fields. Built `JsonFileLayer` implementing `tracing_subscriber::Layer<S>` with `on_new_span` (captures metadata, fields, parent via contextual lookup), `on_record` (merges additional fields), and `on_close` (computes duration, detects root spans, collects full trace tree, writes atomically via NamedTempFile+persist, prunes old files). Wired the layer into `init_tracing()` using `Option<JsonFileLayer>` in the `.with()` chain — tracing-subscriber natively supports optional layers. Added `registry` feature explicitly to workspace tracing-subscriber deps.

Created integration test file with 3 tests: tree structure verification (4 nested spans with parent-child assertions, field capture, timing > 0), pruning behavior (55 dummy files reduced to ≤ 50 after trace write), and multiple root spans producing separate trace files.

## Verification

- `cargo test -p assay-core trace_export` — 3/3 integration tests pass (tree structure, pruning, multiple roots)
- `cargo test -p assay-core -- telemetry` — 4 unit tests + 1 doctest pass
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (fixed one collapsible_if lint)

### Slice-level checks:
- ✅ `cargo test -p assay-core trace_export` — passes
- ⬜ `cargo test -p assay-cli traces` — CLI subcommand not yet implemented (T02/T03)
- ✅ `cargo test -p assay-core telemetry` — passes
- ⬜ `just ready` — not run (full workspace check deferred to final task)

## Diagnostics

- `tracing::debug!` emitted on each trace file write with path and span count
- `tracing::warn!` emitted on write/persist/prune failures with path and error context
- JSON files in `.assay/traces/` are self-contained: array of SpanData with full tree

## Deviations

- Used `chrono::DateTime::parse_from_rfc3339` for duration calculation instead of storing raw `Instant` — RFC 3339 strings are already captured for serialization, and parsing back is reliable for millisecond-level timing.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/telemetry.rs` — Extended with SpanData, FieldVisitor, JsonFileLayer, generate_trace_id; TracingConfig.traces_dir field; init_tracing wires optional layer
- `crates/assay-core/tests/trace_export.rs` — New integration test file with 3 tests
- `Cargo.toml` — Added `registry` feature to tracing-subscriber workspace dep
