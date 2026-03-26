# S04: JSON file trace export and CLI — UAT

**Milestone:** M009
**Written:** 2026-03-25

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All behavior is proven by integration tests using real tracing subscribers with real filesystem I/O. No external services, daemons, or interactive sessions are required. The CLI rendering is verified by unit tests asserting exact output structure. Human inspection of `.assay/traces/` and CLI output provides confirmation but is not required for correctness proof.

## Preconditions

- `just ready` passes (all tests green, no clippy warnings, fmt clean)
- A project with `.assay/` directory exists (for traces_dir wiring test)
- `assay` binary is built (`cargo build -p assay-cli`)

## Smoke Test

Run a gate check on any spec with tracing enabled and confirm a JSON trace file appears:

```bash
cd <project-with-assay>
cargo run -p assay-cli -- gate run <spec-slug>
ls .assay/traces/
# Expected: at least one *.json file
assay traces list
# Expected: table with timestamp, root span, span count
```

## Test Cases

### 1. JsonFileLayer writes correct JSON trace file

1. Install a tracing subscriber with `JsonFileLayer` pointed at a temp dir
2. Create a 3-level span tree: root → child → grandchild with fields
3. Let all spans close
4. Read the JSON file from the temp dir

**Expected:** Exactly one JSON file. Array of 3 SpanData objects. Root has `parent_id: null`. Child has `parent_id` matching root's `span_id`. Grandchild has `parent_id` matching child's `span_id`. All have positive `duration_ms`. Fields captured correctly.

### 2. File pruning keeps at most max_files traces

1. Pre-populate temp dir with 55 dummy JSON files
2. Run one trace through JsonFileLayer with max_files=50
3. Count JSON files in dir

**Expected:** Count ≤ 50. Oldest files removed first (lexicographic order).

### 3. Multiple root spans produce separate trace files

1. Install subscriber with JsonFileLayer
2. Create two separate root spans (not nested) with a delay between them
3. Let both close

**Expected:** Exactly 2 JSON files in the traces dir.

### 4. End-to-end round-trip: write → read → render

1. Write a 4-span tree via real subscriber (orchestration_run → session → gate_eval + merge_propose)
2. Read the JSON file back
3. Verify parent-child relationships match written structure
4. Reconstruct adjacency map (same as CLI show logic)

**Expected:** All 4 spans present with correct hierarchy. Tree can be rendered from parent_id references.

### 5. assay traces list — shows trace files in table

1. Create a traces dir with known JSON files (via unit test helpers)
2. Call handle_list() with the dir

**Expected:** Table printed with columns: ID / Timestamp / Root Span / Spans. One row per file, sorted by filename.

### 6. assay traces show — renders indented span tree

1. Create trace JSON with root + 2 children
2. Call handle_show() with the trace ID

**Expected:** Indented tree output. Root at depth 0, children at depth 1 (2-space indent). Each line shows span name and duration_ms.

### 7. assay traces show — error on missing trace ID

1. Call handle_show() with a nonexistent trace ID

**Expected:** Exit code 1. tracing::error! logged with id and path. No panic.

### 8. assay traces list — graceful on malformed JSON

1. Place a file with invalid JSON content in traces dir
2. Call handle_list()

**Expected:** Malformed file is skipped with tracing::warn!. Valid files still listed.

### 9. traces_dir wired for pipeline subcommands, not Traces subcommand

1. Inspect tracing_config_for(Some(Command::Run(_))) → traces_dir is Some when .assay/ exists
2. Inspect tracing_config_for(Some(Command::Traces { .. })) → traces_dir is None

**Expected:** No self-tracing loop for `assay traces list/show`. Pipeline runs do produce trace files.

## Edge Cases

### Empty traces directory

1. Create empty `.assay/traces/` dir
2. Run `assay traces list`

**Expected:** Table header printed, no data rows, exit 0.

### Traces directory does not exist

1. Run `assay traces list` from a dir with no `.assay/traces/`

**Expected:** tracing::error! logged, exit code 1.

## Failure Signals

- No JSON files in `.assay/traces/` after a `gate run` — JsonFileLayer not wired or traces_dir not set
- `assay traces list` exits non-zero on valid traces dir — scan/parse failure
- `assay traces show <id>` shows flat list instead of indented tree — adjacency map broken
- Negative or zero duration_ms on spans — clock or RFC 3339 parse issue
- More than max_files files accumulate — pruning logic broken

## Requirements Proved By This UAT

- R063 (JSON file trace export) — JsonFileLayer writes structured JSON files per trace; `assay traces list` and `assay traces show` provide CLI inspection. Proven by 4 integration tests (trace_export.rs) and 7 unit tests (traces.rs) all passing.

## Not Proven By This UAT

- R027 (full OTel instrumentation across all stages) — S04 proves the export side; pipeline and orchestration spans (S02/S03) are proven separately
- R064 (OTLP export) — deferred to S05
- R065 (TRACEPARENT propagation) — deferred to S05
- R066 (TUI trace viewer) — deferred to future milestone
- Live runtime behavior: that a real `assay gate run` invocation in a real project produces trace files visible via `assay traces list` — this is manual verification only (automated tests use in-process subscriber)

## Notes for Tester

- Trace file IDs are timestamp-based (`<date>T<hex>.json`) — `assay traces show` takes the stem without `.json`
- The pruning keeps the 50 most recent files (by filename sort order, which equals chronological order due to timestamp prefix)
- `assay traces list` reads from `.assay/traces/` relative to the current working directory
- The self-tracing guard (traces_dir: None for Traces subcommand) means running `assay traces list` does not create new trace files
