---
estimated_steps: 5
estimated_files: 5
---

# T02: CLI `run` subcommand and MCP `run_manifest` tool

**Slice:** S07 — End-to-End Pipeline
**Milestone:** M001

## Description

Wire the pipeline module into both user-facing entry points: the `assay run <manifest.toml>` CLI subcommand and the `run_manifest` MCP tool. The CLI provides human-facing progress output; the MCP tool wraps the sync pipeline in `spawn_blocking` for agent invocation.

## Steps

1. **Create `crates/assay-cli/src/commands/run.rs`:**
   - Define `RunCommand` struct with clap derives: `manifest` positional arg (PathBuf), `--timeout` optional (u64, default 600), `--json` flag for machine-readable output, `--base-branch` optional.
   - Implement `pub fn execute(cmd: &RunCommand)` that:
     - Resolves project_root, assay_dir, specs_dir, worktree_base from config (following existing CLI patterns)
     - Loads manifest via `assay_core::manifest::load(&cmd.manifest)`
     - Constructs `PipelineConfig` with resolved paths and timeout
     - Calls `assay_core::pipeline::run_manifest()` 
     - Prints stage-by-stage progress to stderr (one line per stage completion)
     - Prints final result to stdout: JSON when `--json`, human-readable summary otherwise
     - Returns appropriate exit code: 0 for all sessions succeed, 1 for any pipeline error, 2 for gate failures

2. **Register `run` subcommand in CLI:**
   - Add `pub mod run;` to `crates/assay-cli/src/commands/mod.rs`
   - Add `Run(commands::run::RunCommand)` variant to `Command` enum in `main.rs`
   - Add match arm dispatching to `commands::run::execute()`

3. **Add `run_manifest` MCP tool to `crates/assay-mcp/src/server.rs`:**
   - Define `RunManifestParams` struct with `manifest_path: String` and `timeout_secs: Option<u64>`
   - Add `#[tool(...)]` annotated async method on `AssayServer`
   - Inside: resolve paths, load config, wrap `pipeline::run_manifest()` in `tokio::task::spawn_blocking`
   - Return JSON result with per-session outcomes
   - On pipeline error, return `CallToolResult` with `isError: true` and structured error JSON

4. **Update `assay-mcp/Cargo.toml` if needed** to depend on pipeline types (should already depend on assay-core).

5. **Add tests:**
   - CLI: verify `assay run --help` includes the subcommand (clap compile test)
   - MCP: `RunManifestParams` deserializes correctly from JSON, schema generates without panic
   - Integration: verify `run_manifest` tool appears in the tool router list

## Must-Haves

- [ ] `assay run <manifest.toml>` CLI subcommand parses and dispatches correctly
- [ ] `--timeout` flag configures pipeline timeout (default 600s)
- [ ] `run_manifest` MCP tool exists with correct parameter schema
- [ ] MCP tool wraps sync pipeline call in `spawn_blocking` (per D007)
- [ ] MCP tool is additive — no existing tool signatures modified (per D005)
- [ ] Pipeline errors rendered as structured JSON in MCP responses with `isError: true`

## Verification

- `cargo test -p assay-cli` — CLI tests pass
- `cargo test -p assay-mcp` — MCP tests pass including run_manifest tool
- `just ready` — full suite green

## Observability Impact

- Signals added/changed: CLI prints stage progress lines to stderr during execution. MCP tool returns structured JSON with per-session results including stage timings and errors.
- How a future agent inspects this: MCP `run_manifest` tool returns machine-readable JSON with stage-level detail. CLI `--json` flag provides same structured output for scripting.
- Failure state exposed: CLI exit codes (0/1/2) distinguish success, pipeline error, and gate failure. MCP `isError` flag plus error JSON provides equivalent for agents.

## Inputs

- `crates/assay-core/src/pipeline.rs` — `run_manifest()`, `PipelineConfig`, `PipelineResult`, `PipelineError` (from T01)
- `crates/assay-core/src/manifest.rs` — `load()` for manifest parsing
- `crates/assay-mcp/src/server.rs` — existing MCP tool patterns (spawn_blocking, param structs, domain_error)
- `crates/assay-cli/src/commands/mod.rs` — existing CLI command patterns

## Expected Output

- `crates/assay-cli/src/commands/run.rs` — new CLI run command module
- `crates/assay-cli/src/main.rs` — `Run` variant added to `Command` enum
- `crates/assay-cli/src/commands/mod.rs` — `pub mod run;` added
- `crates/assay-mcp/src/server.rs` — `run_manifest` tool added with param struct and handler
