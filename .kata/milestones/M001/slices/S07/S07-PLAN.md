# S07: End-to-End Pipeline

**Goal:** `assay run <manifest.toml>` and `run_manifest` MCP tool execute the full single-agent pipeline: manifest load → worktree create → harness config generate/write → agent launch → gate evaluate → merge check. Pipeline failures produce structured errors with stage context and recovery guidance.
**Demo:** Run `assay run test-manifest.toml` against a fixture manifest. The CLI sequences through all pipeline stages, reports structured results per stage, and handles failure (timeout, crash, bad exit code) with stage-tagged errors. The `run_manifest` MCP tool does the same via `spawn_blocking`.

## Must-Haves

- `PipelineStage` enum and `PipelineError` struct in `assay-core::pipeline` with stage context, elapsed time, and recovery guidance
- `run_session()` sync function executing one ManifestSession through the full pipeline
- `run_manifest()` sync function iterating sessions and calling `run_session()`
- `launch_agent()` sync subprocess launcher using `std::process::Command` with thread-based timeout and structured exit code mapping
- `HarnessProfile` construction from `ManifestSession` inline overrides + spec context
- `assay run <manifest.toml>` CLI subcommand
- `run_manifest` MCP tool wrapping the pipeline via `spawn_blocking`
- Pipeline failures at every stage produce `PipelineError` with the stage that failed

## Proof Level

- This slice proves: final-assembly (composes all prior slices into the real pipeline)
- Real runtime required: yes (subprocess launch, git worktree, file I/O)
- Human/UAT required: yes (real Claude Code invocation is a manual verification step — automated tests use mock/fixture subprocess)

## Verification

- `cargo test -p assay-core -- pipeline` — pipeline module unit tests covering each stage's success and failure paths
- `cargo test -p assay-mcp -- run_manifest` — MCP tool compilation and parameter struct tests
- `cargo test -p assay-cli` — CLI `run` subcommand integration
- `just ready` — full suite green (fmt, clippy, all tests, deny)

## Observability / Diagnostics

- Runtime signals: `PipelineError` includes `stage: PipelineStage`, `elapsed: Duration`, and `recovery: String` for every failure. Pipeline success returns `PipelineResult` with per-stage timing.
- Inspection surfaces: `PipelineResult` struct is returned as JSON from MCP tool; CLI prints stage-by-stage progress to stderr.
- Failure visibility: On failure, `PipelineError` identifies the exact stage (ManifestLoad, WorktreeCreate, HarnessConfig, AgentLaunch, GateEvaluate, MergeCheck), wraps the underlying `AssayError`, and provides recovery guidance string.
- Redaction constraints: Agent subprocess stdout/stderr may contain secrets — pipeline captures but does not log raw agent output in error messages beyond stderr excerpts for crash diagnosis.

## Integration Closure

- Upstream surfaces consumed:
  - `assay_core::manifest::load()` (S06)
  - `assay_core::worktree::create()` with session linkage (S05)
  - `assay_harness::claude::{generate_config, write_config, build_cli_args}` (S04)
  - `assay_core::work_session::{start_session, record_gate_result, complete_session, abandon_session}` (S01)
  - `assay_core::gate::evaluate_all()` (pre-existing)
  - `assay_core::merge::merge_check()` (pre-existing)
  - `assay_core::spec::load_spec_entry()` (pre-existing)
- New wiring introduced in this slice: Pipeline orchestrator composes all of the above into a single sequenced flow. CLI `run` subcommand and MCP `run_manifest` tool are the two entry points.
- What remains before the milestone is truly usable end-to-end: Manual UAT with a real Claude Code installation and a real spec.

## Tasks

- [x] **T01: Pipeline module with PipelineStage, PipelineError, and run_session orchestrator** `est:45m`
  - Why: This is the core pipeline logic — the orchestrator that sequences manifest → worktree → harness → agent → gate → merge check. Without this, nothing else works.
  - Files: `crates/assay-core/src/pipeline.rs`, `crates/assay-core/src/error.rs`, `crates/assay-core/src/lib.rs`
  - Do: Create `pipeline` module with `PipelineStage` enum (ManifestLoad, WorktreeCreate, HarnessConfig, AgentLaunch, GateEvaluate, MergeCheck), `PipelineError` struct (stage, source error, elapsed, recovery guidance), `PipelineResult` struct (per-stage timing, gate summary, merge check result), `PipelineConfig` struct (timeout, project paths). Implement `run_session()` that: loads spec, creates worktree with session linkage, constructs HarnessProfile from ManifestSession overrides, generates+writes claude config, launches agent subprocess (sync Command with thread-based timeout), evaluates gates, checks merge readiness, manages session lifecycle (start→gate→complete, abandon on failure). Implement `run_manifest()` iterating sessions. Implement `launch_agent()` with timeout+kill+exit-code-mapping. Add unit tests for each stage's success and failure path using mocked/fixture subprocess behavior.
  - Verify: `cargo test -p assay-core -- pipeline` passes all tests; `cargo clippy -p assay-core` clean
  - Done when: Pipeline module compiles, orchestrates the full sequence, and tests cover success path plus at least one failure path per stage (timeout, crash, bad config, etc.)

- [x] **T02: CLI `run` subcommand and MCP `run_manifest` tool** `est:30m`
  - Why: The pipeline needs entry points — CLI for human use, MCP for agent invocation. Without these, the pipeline logic is unreachable.
  - Files: `crates/assay-cli/src/main.rs`, `crates/assay-cli/src/commands/mod.rs`, `crates/assay-cli/src/commands/run.rs`, `crates/assay-mcp/src/server.rs`
  - Do: Add `assay run <manifest>` CLI subcommand with `--timeout` flag, delegating to `pipeline::run_manifest()`. Print stage progress to stderr and final result to stdout (JSON when `--json`). Add `run_manifest` MCP tool with manifest_path param, wrapping pipeline call in `spawn_blocking`. Wire both entry points. Add MCP param struct tests.
  - Verify: `cargo test -p assay-cli` and `cargo test -p assay-mcp -- run_manifest` pass; `just ready` green
  - Done when: `assay run --help` shows the subcommand, MCP tool compiles with correct parameter schema, `just ready` passes

## Files Likely Touched

- `crates/assay-core/src/pipeline.rs` (new)
- `crates/assay-core/src/lib.rs`
- `crates/assay-core/src/error.rs`
- `crates/assay-cli/src/main.rs`
- `crates/assay-cli/src/commands/mod.rs`
- `crates/assay-cli/src/commands/run.rs` (new)
- `crates/assay-mcp/src/server.rs`
