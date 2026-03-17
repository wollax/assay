---
id: T01
parent: S04
milestone: M002
provides:
  - Codex adapter (generate_config, write_config, build_cli_args)
  - TOML-based .codex/config.toml generation via serde
  - Hook advisory text in AGENTS.md for Codex sessions
key_files:
  - crates/assay-harness/src/codex.rs
  - crates/assay-harness/src/snapshots/ (9 codex snapshot files)
key_decisions:
  - sandbox_mode defaults to workspace-write; escalates to danger-full-access only for network/system permissions
  - Hooks rendered as advisory markdown section since Codex lacks native hook support
patterns_established:
  - Codex adapter follows identical structure to claude.rs (CodexConfig struct, generate_config/write_config/build_cli_args)
observability_surfaces:
  - none — pure functions, insta snapshots serve as canonical reference
duration: 10m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Codex adapter with snapshot tests

**Implemented Codex adapter generating `.codex/config.toml` (TOML), `AGENTS.md`, and CLI args from a HarnessProfile, with 12 tests and 9 insta snapshots.**

## What Happened

Created `crates/assay-harness/src/codex.rs` following the `claude.rs` adapter pattern exactly:

- `CodexConfig` struct with pre-serialized `agents_md`, `config_toml`, and optional `model`
- `generate_config()` as a pure function: builds AGENTS.md via `build_prompt()`, appends hook advisory text when hooks are present, serializes `CodexConfigToml` to TOML via the `toml` crate
- `write_config()` writes AGENTS.md (skipped if empty) and `.codex/config.toml` (creating `.codex/` dir)
- `build_cli_args()` produces `["exec", "--full-auto"]` with optional `--model`
- `resolve_sandbox_mode()` maps permissions to sandbox level: `workspace-write` by default, `danger-full-access` for network/system ops
- Hook advisory section appended to AGENTS.md with human-readable event labels and timeout info

Added `toml.workspace = true` to `assay-harness/Cargo.toml` and `pub mod codex;` to `lib.rs`.

## Verification

- `cargo test -p assay-harness -- codex` — 12 tests pass (3 profile snapshots, 1 hook assertion, 3 write_config tempfile, 3 build_cli_args snapshots, 2 sandbox_mode assertions)
- `cargo clippy -p assay-harness -- -D warnings` — clean, no warnings
- `cargo test -p assay-harness` — all 39 harness tests pass (Claude + Codex)
- 9 snapshot files in `crates/assay-harness/src/snapshots/` with `codex` prefix

### Slice-level verification (intermediate — T01 of 3):
- ✅ `cargo test -p assay-harness -- codex` — all Codex adapter tests pass
- ⬜ `cargo test -p assay-harness -- opencode` — not yet implemented (T02)
- ✅ `cargo test -p assay-harness` — 39 tests pass (Claude + Codex)
- ⬜ `just ready` — deferred to T03

## Diagnostics

Inspect snapshot files in `crates/assay-harness/src/snapshots/` to see exact expected Codex config format. Snapshot mismatches produce inline diffs on regression.

## Deviations

- Added 2 extra tests beyond plan: `sandbox_escalation_network` and `sandbox_default_workspace_write` to directly verify sandbox mode logic. Total 12 tests (plan estimated ~10).

## Known Issues

None.

## Files Created/Modified

- `crates/assay-harness/Cargo.toml` — added `toml.workspace = true`
- `crates/assay-harness/src/lib.rs` — added `pub mod codex;`
- `crates/assay-harness/src/codex.rs` — complete Codex adapter (~170 lines impl + ~280 lines tests)
- `crates/assay-harness/src/snapshots/` — 9 new Codex snapshot files
