---
estimated_steps: 5
estimated_files: 4
---

# T04: CLI flag and MCP parameter for conflict resolution

**Slice:** S01 — AI Conflict Resolution
**Milestone:** M003

## Description

Wire the conflict resolver into both user-facing entry points: CLI `--conflict-resolution auto|skip` flag and MCP `orchestrate_run` `conflict_resolution` parameter. When `auto`, compose a handler closure that calls `resolve_conflict()` from T02, pass it to `merge_completed_sessions()`, and set `conflict_resolution_enabled: true` on `MergeRunnerConfig`. When `skip` (default), preserve existing behavior. Run `just ready` to confirm zero regressions.

## Steps

1. Add `--conflict-resolution` flag to `RunCommand` in `crates/assay-cli/src/commands/run.rs`:
   ```rust
   #[arg(long, default_value = "skip", value_parser = parse_conflict_resolution)]
   pub conflict_resolution: ConflictResolutionMode,
   ```
   Define `ConflictResolutionMode` enum (`Auto`, `Skip`) with parser. Update help text examples.

2. Update `execute_orchestrated()` in `run.rs`: when `conflict_resolution` is `Auto`, compose handler closure:
   ```rust
   let config = ConflictResolutionConfig { enabled: true, ..Default::default() };
   let handler = |name: &str, files: &[String], scan: &ConflictScan, dir: &Path| {
       resolve_conflict(name, files, scan, dir, &config)
   };
   ```
   Set `conflict_resolution_enabled: true` on `MergeRunnerConfig`. Pass handler to `merge_completed_sessions()`. When `Skip`, use `default_conflict_handler()` as before.

3. Add `conflict_resolution` optional parameter to `OrchestrateRunParams` in `crates/assay-mcp/src/server.rs`:
   ```rust
   #[serde(default)]
   pub conflict_resolution: Option<String>,
   ```
   Add schemars description. In the `orchestrate_run` handler, parse the value and compose the handler closure the same way as CLI. Default to `skip`.

4. Add CLI tests: verify `--conflict-resolution auto` parses correctly, `--conflict-resolution skip` is default, invalid value produces error. Add MCP test: `OrchestrateRunParams` with `"conflict_resolution": "auto"` deserializes.

5. Run `just ready` — all tests pass, no clippy warnings, format clean.

## Must-Haves

- [ ] `--conflict-resolution auto|skip` flag on `assay run` (default `skip`)
- [ ] CLI `auto` mode composes `resolve_conflict()` handler and sets `conflict_resolution_enabled: true`
- [ ] MCP `orchestrate_run` accepts `conflict_resolution` parameter
- [ ] MCP `auto` mode composes same handler
- [ ] Default behavior unchanged: `skip` uses `default_conflict_handler()`
- [ ] CLI arg parsing tests (valid values + invalid value error)
- [ ] MCP deserialization test
- [ ] `just ready` green

## Verification

- `cargo test -p assay-cli run` — existing + new CLI flag tests pass
- `cargo test -p assay-mcp orchestrate_run` — existing + new param tests pass
- `just ready` — full suite green (fmt, lint, test, deny)

## Observability Impact

- Signals added/changed: None — this task only wires existing signals through CLI/MCP entry points
- How a future agent inspects this: `assay run --help` shows the `--conflict-resolution` flag; MCP schema includes `conflict_resolution` param
- Failure state exposed: Invalid CLI arg produces clap error with valid options listed

## Inputs

- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — `resolve_conflict()` function (T02)
- `crates/assay-core/src/orchestrate/merge_runner.rs` — `MergeRunnerConfig` with `conflict_resolution_enabled` (T03), `default_conflict_handler()`, `merge_completed_sessions()`
- `crates/assay-types/src/orchestrate.rs` — `ConflictResolutionConfig` (T01)
- `crates/assay-cli/src/commands/run.rs` — existing `RunCommand` and `execute_orchestrated()`
- `crates/assay-mcp/src/server.rs` — existing `OrchestrateRunParams` and `orchestrate_run` handler

## Expected Output

- `crates/assay-cli/src/commands/run.rs` — `--conflict-resolution` flag + handler composition + tests
- `crates/assay-mcp/src/server.rs` — `conflict_resolution` parameter + handler composition + test
- `just ready` passing
