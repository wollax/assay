---
estimated_steps: 5
estimated_files: 4
---

# T01: Custom tracing Layer and JSON file writer

**Slice:** S04 — JSON file trace export and CLI
**Milestone:** M009

## Description

Implement a custom `tracing_subscriber::Layer` in `assay_core::telemetry` that captures span lifecycle events (new, record, close) in a thread-safe in-memory map and writes a structured JSON file per trace when the root span closes. Extend `TracingConfig` with `traces_dir` and wire the layer into `init_tracing()`. Create the integration test file with real subscriber assertions.

## Steps

1. Add `traces_dir: Option<PathBuf>` to `TracingConfig` with `Default` returning `None`. Update `TracingConfig::mcp()` to set `None`. Verify existing unit tests still pass.
2. Define `SpanData` struct (name, target, level, parent_id, start_time, end_time, duration_ms, fields, events) and `TraceRecord` (serializable wrapper for the JSON array). Add `serde::Serialize` derives on these types.
3. Implement `JsonFileLayer` struct holding `traces_dir: PathBuf`, `max_files: usize`, `spans: Mutex<HashMap<Id, SpanData>>`. Implement `Layer<S>` for it with `on_new_span` (record metadata + fields + start time + parent), `on_record` (merge fields), `on_close` (compute duration, detect root, collect all spans for this trace, write JSON file atomically, prune old files).
4. Wire `JsonFileLayer` into `init_tracing()`: when `config.traces_dir` is `Some(dir)`, create the directory if needed, construct the layer, and add it to the subscriber via `.with()`. Use `Option<JsonFileLayer>` in the layer chain (tracing-subscriber supports optional layers).
5. Create `crates/assay-core/tests/trace_export.rs` with integration tests: install a subscriber with `JsonFileLayer` pointing at a tempdir, execute instrumented code (nested spans with fields), assert JSON file is created with correct span count, parent-child relationships, timing > 0, and fields present. Test pruning by creating 55 dummy files and verifying only 50 remain after a new trace write.

## Must-Haves

- [ ] `TracingConfig.traces_dir: Option<PathBuf>` field with backward-compatible default `None`
- [ ] `JsonFileLayer` implements `Layer<S> for S: Subscriber + for<'a> LookupSpan<'a>`
- [ ] `on_new_span` captures name, target, level, parent_id, fields, start_time
- [ ] `on_close` computes duration, detects root (no parent), writes JSON file atomically via NamedTempFile+persist
- [ ] JSON file format: array of span records with all required fields
- [ ] File pruning: oldest files deleted when count exceeds max (default 50)
- [ ] `init_tracing()` adds `JsonFileLayer` when `traces_dir` is `Some`
- [ ] Integration test proves JSON file creation with correct tree structure
- [ ] Integration test proves pruning behavior

## Verification

- `cargo test -p assay-core trace_export` — integration tests pass
- `cargo test -p assay-core telemetry` — existing + new unit tests pass
- `cargo clippy --workspace --all-targets -- -D warnings` — clean

## Observability Impact

- Signals added/changed: `tracing::debug!` on trace file write (path, span count); `tracing::warn!` on write failure or prune failure. These events go to the fmt layer, not the JSON file layer (avoiding recursive capture).
- How a future agent inspects this: check `.assay/traces/` directory for JSON files; each file is a self-contained trace
- Failure state exposed: write/prune failures logged with path and error context

## Inputs

- `crates/assay-core/src/telemetry.rs` — existing `init_tracing()` with registry+filter+fmt layer composition
- S01 summary — layer architecture uses `registry().with()` chain; `try_init()` for safe double-init
- S04 research — custom Layer approach, not built-in JSON formatter; `Mutex<HashMap<Id, SpanData>>` for thread safety; root span detection via no-parent heuristic

## Expected Output

- `crates/assay-core/src/telemetry.rs` — extended with `JsonFileLayer`, `SpanData`, `TracingConfig.traces_dir`
- `crates/assay-core/Cargo.toml` — `registry` feature added to tracing-subscriber (already transitively enabled, but explicit is safer)
- `crates/assay-core/tests/trace_export.rs` — new integration test file with ≥3 tests
