---
id: T04
parent: S01
milestone: M003
provides:
  - CLI --conflict-resolution auto|skip flag on assay run (default skip)
  - CLI auto mode composes resolve_conflict() handler with conflict_resolution_enabled=true
  - MCP orchestrate_run conflict_resolution parameter (auto|skip, default skip)
  - MCP auto mode composes same resolve_conflict() handler
  - Fixed pre-existing unused import warning in conflict_resolver.rs (git_raw)
key_files:
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-core/src/orchestrate/conflict_resolver.rs
key_decisions:
  - ConflictResolutionMode enum kept crate-local to assay-cli (not assay-types) — it's a CLI-presentation concern, not a persistence/API type
  - MCP uses Option<String> for conflict_resolution rather than a typed enum to match the existing pattern for failure_policy and merge_strategy in OrchestrateRunParams
  - Handler closure captures ConflictResolutionConfig by move — consistent with how merge_strategy/failure_policy are captured inside spawn_blocking
patterns_established:
  - CLI mode enum + value_parser pattern: ConflictResolutionMode enum with parse_conflict_resolution follows the same pattern as parse_failure_policy and parse_merge_strategy
  - Conditional handler composition: match on mode before merge call, compose appropriate handler closure in each arm — avoids Box<dyn Fn> overhead
observability_surfaces:
  - assay run --help shows --conflict-resolution flag with auto|skip values and description
  - MCP schema includes conflict_resolution parameter with description of auto vs skip behavior
  - Invalid CLI arg produces clap error listing valid options; invalid MCP value returns domain error text
duration: ~25min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T04: CLI flag and MCP parameter for conflict resolution

**Wired `--conflict-resolution auto|skip` into both `assay run` CLI and MCP `orchestrate_run`, composing the AI handler from T02 when `auto` is selected.**

## What Happened

Added `ConflictResolutionMode` enum (`Auto`, `Skip`) to `assay-cli` with a `parse_conflict_resolution` clap value parser, then added the `--conflict-resolution` flag to `RunCommand` (default `skip`). Updated `execute_orchestrated()` to match on the mode: when `Auto`, creates a `ConflictResolutionConfig { enabled: true, ..Default::default() }` and composes a handler closure over `resolve_conflict()`; when `Skip`, uses `default_conflict_handler()` as before. Both arms set `conflict_resolution_enabled` on `MergeRunnerConfig` accordingly.

On the MCP side, added `conflict_resolution: Option<String>` to `OrchestrateRunParams` with a schemars description, parsed it in the `orchestrate_run` handler to a `use_auto_conflict_resolution: bool`, and composed the handler inside the `spawn_blocking` closure via the same conditional pattern. The existing `orchestrate_run_missing_manifest` test struct literal was updated to include the new field.

Also fixed a pre-existing unused import warning (`git_raw`) in `conflict_resolver.rs` that was blocking `just lint`.

## Verification

- `cargo test -p assay-cli run` — 16 tests pass (4 new: auto parses, skip is default, skip explicit, invalid rejects)
- `cargo test -p assay-mcp orchestrate_run` — 8 tests pass (3 new: auto deserializes, skip deserializes, None defaults)
- `just ready` — fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓ — fully green

## Diagnostics

- `assay run --help` shows `--conflict-resolution <CONFLICT_RESOLUTION>` with valid values and description
- MCP schema for `orchestrate_run` includes `conflict_resolution` field with description
- Invalid CLI value: `clap` error with message "invalid conflict resolution mode 'X': expected 'auto' or 'skip'"
- Invalid MCP value: domain error "Invalid conflict_resolution 'X'. Expected 'auto' or 'skip'."
- Handler failures surface via `MergeSessionResult.error` field (from T03 wiring) — ConflictSkipped status with descriptive error

## Deviations

- Fixed unused import `git_raw` in `conflict_resolver.rs` — pre-existing issue from T02 that was blocked by clippy -D warnings in `just lint`. Not in T04 plan but required for `just ready` to pass.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/run.rs` — ConflictResolutionMode enum + parse_conflict_resolution + --conflict-resolution flag + handler composition in execute_orchestrated() + 4 new tests
- `crates/assay-mcp/src/server.rs` — conflict_resolution field on OrchestrateRunParams + parse + handler composition in orchestrate_run + 3 new tests; updated existing test struct literal
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — removed unused git_raw import
