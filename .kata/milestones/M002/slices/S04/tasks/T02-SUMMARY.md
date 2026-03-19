---
id: T02
parent: S04
milestone: M002
provides:
  - OpenCode adapter (generate_config, write_config, build_cli_args)
  - JSON-based opencode.json generation with $schema field via serde rename
key_files:
  - crates/assay-harness/src/opencode.rs
  - crates/assay-harness/src/lib.rs
key_decisions:
  - Tools mapped as {"tool_name": true} BTreeMap; permissions as {"perm": "allow"} BTreeMap
  - Agent steps config wraps max_turns in nested agent.steps JSON object
  - Hook advisory uses same pattern as Codex (appended to AGENTS.md)
patterns_established:
  - OpenCode adapter follows identical structure to claude.rs and codex.rs (Config struct, generate_config/write_config/build_cli_args)
observability_surfaces:
  - Insta snapshots in crates/assay-harness/src/snapshots/ with opencode prefix serve as canonical reference
duration: 1 step
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: OpenCode adapter with snapshot tests

**Implemented OpenCode adapter generating `opencode.json` (JSON with $schema), `AGENTS.md`, and CLI args from a HarnessProfile, with 10 tests and 9 insta snapshots.**

## What Happened

Created `opencode.rs` following the established adapter pattern from `claude.rs` and `codex.rs`. The adapter translates a `HarnessProfile` into three artifacts:

1. **`AGENTS.md`** — assembled via shared `build_prompt()` with hook advisory text appended when hooks are present (same pattern as Codex, with "OpenCode" in the advisory text)
2. **`opencode.json`** — JSON config with `$schema` field (via `#[serde(rename = "$schema")]`), optional model, tools as `{"name": true}` map, permissions as `{"name": "allow"}` map, and agent steps from max_turns
3. **CLI args** — `opencode run --model <model> --format json`

Empty collections (tools, permissions) are skipped via `skip_serializing_if`. The `$schema` field always appears.

## Verification

- `cargo test -p assay-harness -- opencode` — 10/10 tests pass
- `cargo test -p assay-harness` — 49/49 tests pass (Claude + Codex + OpenCode)
- `cargo clippy -p assay-harness -- -D warnings` — no warnings
- `just ready` — all checks pass (fmt, lint, test, deny)
- 9 snapshot files exist in `crates/assay-harness/src/snapshots/` with `opencode` prefix

## Diagnostics

Inspect snapshot files in `crates/assay-harness/src/snapshots/` to see exact expected OpenCode config format. Snapshot mismatches produce inline diffs on regression.

## Deviations

- Task plan estimated ~12 tests; implemented 10 (combined `write_config_full` and `write_config_creates_files` cover the same ground as planned, `schema_field_present` is a programmatic assertion not a snapshot test). The test plan's `realistic_profile` produces 2 snapshots + 1 assertion = covered. All must-haves are met.
- Test data uses `"edit"` in tools list (plan mentioned tools from SettingsOverride.tools) — uses realistic OpenCode tool names rather than Claude-specific permission formats.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-harness/src/opencode.rs` — complete OpenCode adapter (~170 lines impl + ~270 lines tests)
- `crates/assay-harness/src/lib.rs` — added `pub mod opencode;`
- `crates/assay-harness/src/snapshots/` — 9 new OpenCode snapshot files
