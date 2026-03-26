# S04: JSON file trace export and CLI — Research

**Date:** 2026-03-24
**Domain:** tracing-subscriber custom layers, JSON trace persistence, CLI rendering
**Confidence:** HIGH

## Summary

S04 adds two capabilities: (1) a custom `tracing-subscriber` Layer that captures span lifecycle events and writes them as JSON trace files to `.assay/traces/`, and (2) CLI commands `assay traces list` and `assay traces show <id>` to inspect those files.

The primary design decision is the trace file format. `tracing-subscriber`'s built-in `json` feature emits NDJSON (one event per line) — good for log aggregation but it only captures **events**, not span open/close lifecycle. For a span tree with timing (which `assay traces show` needs to render), we need a custom `Layer` that tracks `on_new_span`, `on_enter`, `on_exit`, `on_close` and writes a structured JSON document per trace. This is a well-understood pattern in the tracing ecosystem (~100 lines of Layer implementation).

Recommendation: Write a custom `Layer` in `assay_core::telemetry` that collects span lifecycle data in-memory (keyed by span ID), serializes to a JSON file on root span close, and stores it under `.assay/traces/<run_id>.json`. The CLI commands are thin readers over these files. No new crate deps needed — `serde_json` and `tracing-subscriber` (with added `registry` feature for `LookupSpan`) are already available.

## Recommendation

**Custom `Layer` approach, NOT `tracing-subscriber`'s built-in `json` formatter.**

The built-in `fmt::format::Json` emits NDJSON event logs — every `tracing::info!()` becomes a line. This is useful for log shipping but does NOT capture span timing (enter/exit/close) which is the core requirement for `assay traces show <id>` tree rendering. A custom Layer that implements `on_new_span`/`on_close`/`on_record` produces a single structured JSON document per trace with the full span tree, timing, and parent-child relationships.

**File format**: One JSON file per trace, containing an array of span records. Each span record has: `name`, `target`, `level`, `parent_id` (nullable), `start_time`, `end_time`, `duration_ms`, `fields` (key-value map), and `events` (child events within the span). This is simple, self-contained, and renderable as a tree without post-processing.

**Integration with `init_tracing()`**: Add an optional `traces_dir: Option<PathBuf>` field to `TracingConfig`. When set, `init_tracing()` adds the JSON file layer to the subscriber stack alongside the existing fmt layer. Call sites pass `.assay/traces/` when the `.assay/` directory exists. S01's forward intelligence says: "S04/S05 add layers by extending the `with()` chain in `init_tracing()`, not by creating a new subscriber."

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic file writes | `tempfile::NamedTempFile` + `persist()` pattern (used in 16+ files across assay-core) | Battle-tested atomic write; prevents corrupt trace files on crash |
| Run ID generation | `generate_run_id()` in `history/mod.rs` — timestamp + random suffix | Consistent naming with existing history files; chronological sorting |
| Span tree rendering in CLI | Simple recursive indented print (hand-roll, but minimal) | ~30 lines; no crate needed for indented tree display |
| tracing Layer trait | `tracing_subscriber::Layer` — the standard interface | This IS the tracing ecosystem's extension point; no alternative |

## Existing Code and Patterns

- `crates/assay-core/src/telemetry.rs` — Current `init_tracing()` with `registry().with(filter).with(fmt_layer)`. Extend this with `.with(json_file_layer)` when `traces_dir` is Some. The `try_init()` pattern means only one subscriber init takes effect — no layering conflicts.
- `crates/assay-core/src/history/mod.rs` — `generate_run_id()` for timestamp-based IDs, `save()` for atomic JSON file writes via NamedTempFile. Reuse the ID format and write pattern. `list()` for scanning JSON files in a directory — same pattern applies to `.assay/traces/`.
- `crates/assay-cli/src/commands/history.rs` — Example of a CLI subcommand reading JSON files from `.assay/` and rendering them. Follow the same `handle()` dispatch pattern.
- `crates/assay-cli/src/main.rs` — `tracing_config_for()` determines per-subcommand tracing config. The `Traces` subcommand uses default tracing config (info level) but the trace file layer itself should capture all levels to avoid missing debug spans.
- `crates/assay-core/tests/pipeline_spans.rs` — Shows how `tracing-test` with `#[traced_test]` captures span names for assertion. S04 tests should use a similar pattern but assert on the JSON file output instead.

## Constraints

- **`tracing-subscriber` needs `registry` feature** — The `LookupSpan` trait (needed by custom Layer to access span data) requires the `registry` feature. Currently workspace has `["fmt", "env-filter"]`. Must add `"registry"` to workspace features. This is a zero-cost change — `registry` is already transitively enabled by `fmt`.
- **`tracing-subscriber` needs `json` feature** — NOT for the built-in json formatter (we're writing a custom Layer), but `"json"` pulls in `tracing-serde` which provides `AsSerde` for span data serialization. Actually, we should NOT use the `json` feature — we'll serialize span data ourselves via `serde_json` which is already a dep. Avoids pulling in `tracing-serde` unnecessarily.
- **Span data is only accessible inside Layer callbacks** — `on_new_span` receives the `Attributes` (fields), `on_close` signals span completion. We must store span data in a thread-safe structure (e.g., `DashMap` or `Mutex<HashMap>`) keyed by `span::Id`.
- **Thread safety** — Pipeline spans cross threads (D017: `std::thread::scope`). `tracing` handles span propagation natively, but our in-memory store must be `Send + Sync`. A `Mutex<HashMap<Id, SpanData>>` is sufficient — contention is negligible at this scale (tens to hundreds of spans per trace).
- **Trace boundary** — Need to define what constitutes "one trace." Natural boundary: when the root span (e.g., `pipeline::run_session` or `pipeline::run_manifest`) closes, flush all collected spans to a JSON file. For orchestration (S05 territory), the orchestration root span serves the same role.
- **Zero-trait convention (D001)** — The custom Layer necessarily implements the `tracing_subscriber::Layer` trait. This is an exception allowed because `Layer` is an external framework extension point, not a domain abstraction. No domain traits are introduced.
- **File pruning** — Follow the same max-history pattern as `history/mod.rs` for `.assay/traces/`. Default max: 50 trace files. Configurable later.
- **`init_tracing()` signature change** — Adding `traces_dir` to `TracingConfig` changes the struct. All existing callers pass `TracingConfig::default()` or `TracingConfig::mcp()` — both need updating. Use `Option<PathBuf>` defaulting to `None` so existing callers need minimal changes.

## Common Pitfalls

- **Span ID reuse across threads** — `tracing` span IDs are per-subscriber and may be reused after a span closes. The Layer must remove span data from its map in `on_close` AFTER flushing, not before. Race condition: if span IDs are reused quickly, a HashMap keyed by Id could overwrite data. Mitigation: remove from map only in `on_close` (the final lifecycle event).
- **Root span detection** — No built-in way to mark a span as "root." Heuristic: a span with no parent (i.e., `attrs.parent().is_none()` and no current span in the dispatch context) is a root. When the root closes, flush the entire trace. If multiple root spans exist in a process lifetime (e.g., CLI running multiple pipeline executions), each root produces its own trace file.
- **Memory growth for long-running processes** — The TUI is long-lived. If trace collection is always-on, span data accumulates. Mitigation: flush and clear on root span close. Between root spans, no data is retained. MCP server is also long-lived but each tool invocation is bounded.
- **`on_close` called on subscriber drop, not just normal close** — If the process exits without cleanly closing spans, `on_close` may not fire for all spans. The `TracingGuard` drop handler should signal the Layer to flush any in-progress trace data. Alternatively, accept that abnormal exits lose the current trace — consistent with how the non-blocking writer already handles this.
- **CLI `traces` subcommand must NOT init the trace file layer** — When running `assay traces list`, we don't want to create a trace file for the CLI invocation itself. The `tracing_config_for()` function should return config with `traces_dir: None` for the `Traces` subcommand.

## Open Risks

- **Large trace files for orchestration** — A 10-session DAG orchestration with 6 pipeline stages each could produce hundreds of spans. A single trace file may grow to 100KB+. Acceptable for local inspection; revisit if file size becomes a concern.
- **tracing-subscriber type signature complexity** — Composing multiple layers with generics can produce complex type signatures that are hard to work with. The existing `init_tracing()` uses `registry().with(filter).with(fmt_layer)` — adding a third `.with(json_layer)` should work but may require `Box<dyn Layer<...>>` if the concrete types diverge (boxed layers carry minor overhead).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| tracing-subscriber | N/A | No agent skill exists; crate is well-documented and patterns are clear from existing code |
| Ratatui | swiftui skill installed but not relevant | N/A — no TUI work in S04 |

## Sources

- `tracing-subscriber` 0.3.22 source code — `Layer` trait in `layer/mod.rs`, JSON format in `fmt/format/json.rs` — confirms events-only output, no span lifecycle in built-in JSON
- S01 forward intelligence — "S04/S05 add layers by extending the `with()` chain in `init_tracing()`"
- `crates/assay-core/src/history/mod.rs` — `generate_run_id()`, atomic write pattern, list/load pattern reusable for traces
- Workspace `Cargo.toml` — `tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }` — needs no new features (registry already transitively enabled)
- D128 (Dual export) — JSON file export always active when `.assay/` exists
- D129 (Telemetry in assay-core) — custom Layer lives in `assay_core::telemetry`, not a new crate
