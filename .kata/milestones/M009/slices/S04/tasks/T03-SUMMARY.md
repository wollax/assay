---
id: T03
parent: S04
milestone: M009
provides:
  - traces_dir wired into tracing_config_for() for Run/Gate/Context subcommands in assay-cli/src/main.rs
  - End-to-end integration test: write spans via real subscriber → read JSON file → verify tree → reconstruct adjacency map for CLI rendering
  - just ready green across full workspace
key_files:
  - crates/assay-cli/src/main.rs
  - crates/assay-core/tests/trace_export.rs
key_decisions:
  - "traces_dir set to Some(assay_dir.join('traces')) only when .assay/ dir exists — avoids creating dirs on non-project invocations"
  - "Traces subcommand falls through to TracingConfig::default() (traces_dir: None) — no self-tracing loop"
patterns_established:
  - "End-to-end round-trip test pattern: with_json_layer writes spans → read_all_traces reads files → assert parent-child relationships → reconstruct adjacency map (same logic as CLI show)"
observability_surfaces:
  - ".assay/traces/ directory — trace files written by Run/Gate/Context subcommands"
  - "assay traces list / assay traces show — CLI inspection of trace files"
duration: ~30min
verification_result: passed
completed_at: 2026-03-25
blocker_discovered: false
---

# T03: Wire traces_dir into CLI and end-to-end integration test

**Confirmed traces_dir wiring in tracing_config_for() and added end-to-end write→read→render round-trip integration test; just ready passes.**

## What Happened

`tracing_config_for()` in `crates/assay-cli/src/main.rs` was already wired during T02 recovery to set `traces_dir: Some(assay_dir.join("traces"))` for `Command::Run(_) | Command::Gate { .. } | Command::Context { .. }`. The wiring checks that `.assay/` directory exists before setting traces_dir (avoids creating traces dirs for non-project invocations).

Added `trace_export_end_to_end_write_read_render` integration test to `crates/assay-core/tests/trace_export.rs` proving the full loop:
1. **Write phase**: real `JsonFileLayer` subscriber captures 4-span tree (orchestration_run → session → gate_eval + merge_propose) with fields on each span
2. **Read phase**: exactly one JSON file in traces dir; trace ID is non-empty and timestamp-based; all 4 spans deserialized correctly
3. **Verify phase**: parent-child relationships match written structure; field values captured; positive duration_ms on all spans
4. **Render phase**: adjacency map (`HashMap<Option<u64>, Vec<&SpanData>>`) reconstructed from parent_id — same logic as `assay traces show` — proves tree can be rendered

Ran `cargo fmt --all` to fix line-length violations introduced by test code, then confirmed `just ready` passes.

## Verification

- `cargo test -p assay-core trace_export` — 4/4 tests pass (tree structure, pruning, multiple roots, end-to-end round-trip)
- `just ready` — all checks pass (fmt, clippy, test, deny)

### Slice-level checks:
- ✅ `cargo test -p assay-core trace_export` — passes (4 tests)
- ✅ `cargo test -p assay-cli traces` — passes (7 tests)
- ✅ `cargo test -p assay-core telemetry` — passes
- ✅ `just ready` — green

## Diagnostics

- `tracing::debug!` events from JsonFileLayer confirm trace file writes
- `.assay/traces/` directory is the authoritative inspection surface after a Run/Gate/Context invocation

## Deviations

- `tracing_config_for()` wiring was already present from T02 partial recovery. T03 only needed the integration test.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/tests/trace_export.rs` — Added `read_all_traces()` helper and `trace_export_end_to_end_write_read_render` integration test
- `crates/assay-cli/src/main.rs` — traces_dir wiring confirmed (no changes needed)
