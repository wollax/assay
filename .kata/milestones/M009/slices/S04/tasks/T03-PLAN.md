---
estimated_steps: 4
estimated_files: 3
---

# T03: Wire traces_dir into CLI and end-to-end integration test

**Slice:** S04 — JSON file trace export and CLI
**Milestone:** M009

## Description

Complete the wiring: CLI subcommands that run pipelines (Run, Gate, etc.) set `traces_dir` so trace files are produced automatically. Add an end-to-end integration test that exercises the full write → list → show cycle. Verify `just ready` passes.

## Steps

1. Update `tracing_config_for()` in `crates/assay-cli/src/main.rs` to set `traces_dir: Some(assay_dir.join("traces"))` for subcommands that run instrumented work (Run, Gate, Context). Other subcommands (Traces, Init, Spec, Milestone, Plan, History, Pr, Harness, Worktree, Checkpoint, Mcp) keep `traces_dir: None`. This requires detecting the `.assay/` dir at config time — use `project_root().ok().map(|r| assay_dir(&r).join("traces"))` and only set it when the dir exists.
2. Add an end-to-end test in `crates/assay-core/tests/trace_export.rs` that: creates a subscriber with `JsonFileLayer` in a tempdir, executes nested instrumented functions (simulating a pipeline run with named spans), asserts the JSON file exists, deserializes it, validates the tree structure (root → child → grandchild), validates timing fields are non-zero, validates fields are captured.
3. Ensure the `SpanData` types in `telemetry.rs` derive both `Serialize` and `Deserialize` so the CLI can read them back. If not already done in T01, add `Deserialize` derive. Make the span record types `pub` so `traces.rs` in assay-cli can import them.
4. Run `just ready` and fix any issues. Verify: zero clippy warnings, all tests pass, format clean.

## Must-Haves

- [ ] Pipeline-running subcommands (Run, Gate, Context) set `traces_dir` when `.assay/` exists
- [ ] Non-pipeline subcommands (Traces, Init, Spec, etc.) do NOT set `traces_dir`
- [ ] End-to-end test proves write → deserialize → tree structure validation
- [ ] `SpanData` types are `pub` and derive `Deserialize` for CLI consumption
- [ ] `just ready` passes clean

## Verification

- `cargo test -p assay-core trace_export` — all integration tests pass
- `cargo test -p assay-cli` — all CLI tests pass
- `just ready` — green (fmt, lint, test, deny)

## Observability Impact

- Signals added/changed: None (wiring only)
- How a future agent inspects this: `assay traces list` after a `gate run` or `assay run` to verify trace files are being created
- Failure state exposed: if traces_dir detection fails (no .assay/), traces are silently not written — correct behavior since non-project contexts shouldn't produce traces

## Inputs

- `crates/assay-core/src/telemetry.rs` — `JsonFileLayer`, `SpanData`, `TracingConfig` from T01
- `crates/assay-cli/src/commands/traces.rs` — list/show handlers from T02
- `crates/assay-cli/src/main.rs` — `tracing_config_for()` function

## Expected Output

- `crates/assay-cli/src/main.rs` — `tracing_config_for()` updated to set traces_dir for pipeline subcommands
- `crates/assay-core/tests/trace_export.rs` — additional end-to-end integration test
- `crates/assay-core/src/telemetry.rs` — `SpanData` types made pub with Deserialize (if not already)
