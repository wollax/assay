---
id: S04
parent: M002
milestone: M002
provides:
  - Codex adapter (generate_config, write_config, build_cli_args) with TOML config generation
  - OpenCode adapter (generate_config, write_config, build_cli_args) with JSON config generation
  - Hook advisory text pattern for adapters without native hook support
requires:
  - slice: M001/S04
    provides: Claude Code adapter pattern (generate_config/write_config/build_cli_args structure)
  - slice: M001/S03
    provides: build_prompt() and merge_settings() from assay-harness
affects:
  - M002/S05
key_files:
  - crates/assay-harness/src/codex.rs
  - crates/assay-harness/src/opencode.rs
  - crates/assay-harness/src/lib.rs
  - crates/assay-harness/Cargo.toml
  - crates/assay-harness/src/snapshots/ (18 new snapshot files)
key_decisions:
  - Codex sandbox_mode defaults to workspace-write; escalates to danger-full-access only for network/system permissions
  - Hooks rendered as advisory markdown appended to AGENTS.md since Codex/OpenCode lack native hook support
  - OpenCode tools mapped as BTreeMap<String, bool>; permissions as BTreeMap<String, String> with "allow" values
  - OpenCode config includes $schema field via serde rename attribute
patterns_established:
  - All three adapters (Claude, Codex, OpenCode) share identical module structure: Config struct, generate_config(), write_config(), build_cli_args()
  - Hook advisory text pattern for agents without native hook/lifecycle support
observability_surfaces:
  - Insta snapshots in crates/assay-harness/src/snapshots/ serve as canonical reference for all adapter output formats
drill_down_paths:
  - .kata/milestones/M002/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M002/slices/S04/tasks/T02-SUMMARY.md
  - .kata/milestones/M002/slices/S04/tasks/T03-SUMMARY.md
duration: ~25m
verification_result: passed
completed_at: 2026-03-17
---

# S04: Codex & OpenCode Adapters

**Codex and OpenCode harness adapters generating valid config from HarnessProfile — 22 new tests, 18 new snapshots, all three adapters consistent.**

## What Happened

Built two new harness adapters following the Claude Code adapter pattern established in M001/S04:

**T01 — Codex adapter:** Created `codex.rs` with `CodexConfig` struct, `generate_config()` producing `.codex/config.toml` (TOML via serde + toml crate) and `AGENTS.md`, `write_config()` for disk persistence, and `build_cli_args()` producing `["exec", "--full-auto"]`. Added `resolve_sandbox_mode()` mapping permissions to Codex sandbox levels (workspace-write default, danger-full-access for network/system). Hooks rendered as advisory markdown section in AGENTS.md since Codex lacks native hook/lifecycle support. 12 tests with 9 insta snapshots.

**T02 — OpenCode adapter:** Created `opencode.rs` with `OpenCodeConfig` struct, `generate_config()` producing `opencode.json` (JSON with `$schema` field via `#[serde(rename = "$schema")]`) and `AGENTS.md`, `write_config()`, and `build_cli_args()` producing `["opencode", "run"]` with optional model/format flags. Tools mapped as `{"name": true}` BTreeMap, permissions as `{"name": "allow"}` BTreeMap, agent steps from max_turns. Same hook advisory pattern as Codex. 10 tests with 9 insta snapshots.

**T03 — Cross-adapter validation:** Ran `just ready` — all checks passed on first run with zero fixes needed. 49 total harness tests (Claude 10, Codex 12, OpenCode 10, prompt/settings 17), 30 snapshot files.

## Verification

- `cargo test -p assay-harness` — 49 tests pass (Claude + Codex + OpenCode + prompt/settings)
- `ls crates/assay-harness/src/snapshots/ | wc -l` — 30 snapshot files (12 Claude + 9 Codex + 9 OpenCode)
- `just ready` — all four checks pass (fmt, lint, test, deny)
- All three adapters use `build_prompt()` for AGENTS.md content assembly
- All three adapters follow identical module structure: Config struct, generate_config(), write_config(), build_cli_args()

## Requirements Advanced

- R024 — Both Codex and OpenCode adapters now generate valid harness config from HarnessProfile, locked by snapshot tests

## Requirements Validated

- R024 — Snapshot tests + structural assertions prove all three adapters (Claude, Codex, OpenCode) generate correct config from the same HarnessProfile input. 30 snapshots lock the contract.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T01 added 2 extra tests (sandbox mode assertions) beyond the plan estimate, bringing total to 12 instead of ~10.
- T02 implemented 10 tests instead of ~12 — combined coverage approaches cover the same ground with fewer tests.

## Known Limitations

- Codex and OpenCode hook mapping is advisory-only (markdown text in AGENTS.md) — these agents lack native hook/lifecycle support. Claude Code is the only adapter with real hook file generation.
- Adapters are pure config generators — runtime invocation and validation against real Codex/OpenCode agents is not tested (deferred to S05/S06 and manual UAT).

## Follow-ups

- S05 wires CLI dispatch: `assay harness generate codex|opencode` will call these adapters
- S06 integrates adapters into orchestrator pipeline for multi-agent config generation

## Files Created/Modified

- `crates/assay-harness/Cargo.toml` — added `toml.workspace = true`
- `crates/assay-harness/src/lib.rs` — added `pub mod codex;` and `pub mod opencode;`
- `crates/assay-harness/src/codex.rs` — complete Codex adapter (~170 lines impl + ~280 lines tests)
- `crates/assay-harness/src/opencode.rs` — complete OpenCode adapter (~170 lines impl + ~270 lines tests)
- `crates/assay-harness/src/snapshots/` — 18 new snapshot files (9 Codex + 9 OpenCode)

## Forward Intelligence

### What the next slice should know
- All three adapters share identical function signatures: `generate_config(&HarnessProfile, &Path) -> Result<XConfig>`, `write_config(&XConfig, &Path) -> Result<()>`, `build_cli_args(&XConfig) -> Vec<String>`. S05 CLI dispatch can use a simple match on adapter name.
- The `toml` crate is now a workspace dependency — available for any crate that needs it.

### What's fragile
- OpenCode config format is based on research, not official schema validation — if OpenCode changes their config format, snapshots will need updating. Same applies to Codex TOML format.

### Authoritative diagnostics
- `crates/assay-harness/src/snapshots/` — any snapshot mismatch produces an inline diff showing exact deviation. This is the first place to look when adapter output changes.

### What assumptions changed
- No assumptions changed — the adapter pattern from M001/S04 transferred cleanly to both new adapters without friction.
