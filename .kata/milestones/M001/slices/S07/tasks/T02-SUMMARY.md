---
id: T02
parent: S07
milestone: M001
provides:
  - CLI `assay run <manifest.toml>` subcommand with --timeout, --json, --base-branch flags
  - MCP `run_manifest` tool with manifest_path and timeout_secs params, spawn_blocking wrapper
  - Concrete harness writer composing assay_harness::claude functions at both call sites
key_files:
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - "Harness writer composition (claude::generate_config + write_config + build_cli_args) is identical in CLI and MCP — kept inline at each call site rather than extracting a shared function, since it's 3 lines and the two contexts have different error handling needs."
  - "CLI exit codes: 0 = all succeed, 1 = any pipeline error, 2 = gate failures or merge conflicts. MCP uses isError flag on the CallToolResult instead."
patterns_established:
  - "CLI run command uses same resolve_dirs pattern as worktree commands (project_root → config → specs_dir + worktree_base)"
  - "MCP run_manifest wraps sync pipeline in spawn_blocking per D007, same pattern as gate_run and gate_evaluate"
observability_surfaces:
  - "CLI prints stage-by-stage progress to stderr during execution"
  - "CLI --json flag returns structured RunResponse with per-session outcomes, stage timings, and error details"
  - "MCP run_manifest returns structured JSON with identical schema to CLI --json"
  - "CLI exit codes (0/1/2) distinguish success, pipeline error, and gate/merge failure"
  - "MCP isError flag + error JSON for agent-consumable failure signaling"
duration: ~20 minutes
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T02: CLI `run` subcommand and MCP `run_manifest` tool

**Wired the pipeline module into both user-facing entry points: `assay run <manifest.toml>` CLI subcommand and `run_manifest` MCP tool with full structured output.**

## What Happened

Created the CLI `run` subcommand with clap derives for `manifest` (positional PathBuf), `--timeout` (u64, default 600), `--json` (machine-readable output), and `--base-branch` (optional). The execute function resolves project paths from config, loads the manifest, constructs PipelineConfig, and calls `run_manifest()` with a concrete harness writer that composes `assay_harness::claude::{generate_config, write_config, build_cli_args}`.

Added `run_manifest` MCP tool with `RunManifestParams` (manifest_path, timeout_secs) and `#[tool(...)]` annotation. The sync pipeline is wrapped in `tokio::task::spawn_blocking` per D007. Returns structured JSON with per-session outcomes including stage timings and error details.

Both entry points use the same harness writer pattern (dependency-inverted via the `HarnessWriter` function parameter from T01) to compose the concrete claude adapter.

Added `assay-harness` as a dependency to both `assay-cli` and `assay-mcp` Cargo.toml files (already in workspace deps). Added `serde` to `assay-cli` for JSON serialization of run responses.

## Verification

- `cargo test -p assay-cli` — 4 tests pass (clap parsing, JSON serialization of success and error responses)
- `cargo test -p assay-mcp -- run_manifest` — 5 tests pass (param deserialization, schema generation, tool router listing, missing manifest error handling)
- `cargo test -p assay-core -- pipeline` — 18 tests pass (pipeline module from T01)
- `cargo run --bin assay -- run --help` — help text displays correctly with all flags
- `just ready` — full suite green (fmt, clippy, all tests, deny)

## Diagnostics

- CLI: `assay run manifest.toml --json` returns structured JSON with `sessions[]` and `summary` fields
- CLI: stderr shows stage-by-stage progress lines during execution
- CLI: exit code 0 = all succeed, 1 = pipeline error, 2 = gate/merge failure
- MCP: `run_manifest` tool returns JSON with same structure; `isError: true` when any session errored
- MCP: `RunManifestError` includes `stage`, `message`, `recovery`, `elapsed_secs` per failed session

## Deviations

None.

## Known Issues

- Pre-existing flaky test `session_create_happy_path` in assay-mcp occasionally fails under parallel test execution (passes in isolation). Not related to this task.

## Files Created/Modified

- `crates/assay-cli/src/commands/run.rs` — New CLI run command module with RunCommand struct, response types, and execute function
- `crates/assay-cli/src/commands/mod.rs` — Added `pub mod run;`
- `crates/assay-cli/src/main.rs` — Added `Run(commands::run::RunCommand)` variant and match arm
- `crates/assay-cli/Cargo.toml` — Added `assay-harness` and `serde` dependencies
- `crates/assay-mcp/src/server.rs` — Added `RunManifestParams`, response structs, and `run_manifest` tool method with 5 tests
- `crates/assay-mcp/Cargo.toml` — Added `assay-harness` dependency
