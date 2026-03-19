# S04: Codex & OpenCode Adapters

**Goal:** `assay harness generate codex` and `assay harness generate opencode` produce valid harness-specific config from a HarnessProfile, following the same adapter pattern as Claude Code.
**Demo:** Running `cargo test -p assay-harness` shows all adapter snapshot tests passing for Codex (TOML config + AGENTS.md) and OpenCode (JSON config + AGENTS.md), alongside the existing Claude Code tests.

## Must-Haves

- `codex.rs` adapter: `generate_config()`, `write_config()`, `build_cli_args()` — pure functions, zero traits
- `opencode.rs` adapter: `generate_config()`, `write_config()`, `build_cli_args()` — pure functions, zero traits
- Both adapters reuse `build_prompt()` for AGENTS.md content
- Codex config generates `.codex/config.toml` (TOML via `toml` crate) and `AGENTS.md`
- OpenCode config generates `opencode.json` (JSON with `$schema`) and `AGENTS.md`
- Hooks mapped to AGENTS.md advisory text when present (Codex/OpenCode lack native hook support)
- Insta snapshot tests for all generated artifacts (realistic, minimal, hooks-no-model profiles)
- `write_config()` tempfile tests for both adapters
- `build_cli_args()` snapshot tests for both adapters
- `lib.rs` updated with `pub mod codex;` and `pub mod opencode;`
- `toml` workspace dependency added to `assay-harness/Cargo.toml`
- `just ready` passes

## Proof Level

- This slice proves: contract (adapter output fidelity via snapshot tests and structural assertions)
- Real runtime required: no (adapters are pure config generators; runtime use is S05/S06)
- Human/UAT required: no (snapshot tests lock the contract)

## Verification

- `cargo test -p assay-harness -- codex` — all Codex adapter tests pass (snapshots + write + CLI args)
- `cargo test -p assay-harness -- opencode` — all OpenCode adapter tests pass (snapshots + write + CLI args)
- `cargo test -p assay-harness` — all harness tests pass (Claude + Codex + OpenCode, ~55+ tests)
- `just ready` — full workspace passes (fmt, lint, test, deny)

## Observability / Diagnostics

- Runtime signals: None — adapters are pure functions with no runtime state
- Inspection surfaces: Insta snapshots in `crates/assay-harness/src/snapshots/` serve as the canonical reference for expected output
- Failure visibility: Snapshot mismatches produce inline diffs showing exact deviation; compile errors surface missing field handling (D011 pattern)
- Redaction constraints: None

## Integration Closure

- Upstream surfaces consumed: `HarnessProfile`, `SettingsOverride`, `HookContract`, `HookEvent`, `PromptLayer` from `assay-types/src/harness.rs`; `build_prompt()` from `assay-harness/src/prompt.rs`
- New wiring introduced in this slice: `pub mod codex` and `pub mod opencode` in `assay-harness/src/lib.rs`; `toml` dep in `assay-harness/Cargo.toml`
- What remains before the milestone is truly usable end-to-end: S05 wires CLI dispatch (`assay harness generate codex|opencode`); S06 integrates with orchestrator pipeline

## Tasks

- [x] **T01: Codex adapter with snapshot tests** `est:45m`
  - Why: Delivers the Codex half of R024 — generates `.codex/config.toml`, `AGENTS.md`, and CLI args from a HarnessProfile
  - Files: `crates/assay-harness/src/codex.rs`, `crates/assay-harness/src/lib.rs`, `crates/assay-harness/Cargo.toml`, `crates/assay-harness/src/snapshots/` (new snapshot files)
  - Do: Add `toml.workspace = true` to assay-harness deps. Create `codex.rs` mirroring `claude.rs` structure: `CodexConfig` struct, `generate_config()`, `write_config()`, `build_cli_args()`. Map settings to Codex TOML format (`model`, `approval_policy = "full-auto"`, `sandbox_mode`). Map hooks to AGENTS.md advisory text. Register module in `lib.rs`. Write tests: 3 profile snapshots (realistic, minimal, hooks-no-model) × config artifacts, 3 write tests, 3 CLI arg tests.
  - Verify: `cargo test -p assay-harness -- codex` passes; `cargo clippy -p assay-harness` clean
  - Done when: ~15 Codex tests pass including insta snapshots, write_config tempfile tests, and CLI arg tests

- [x] **T02: OpenCode adapter with snapshot tests** `est:45m`
  - Why: Delivers the OpenCode half of R024 — generates `opencode.json`, `AGENTS.md`, and CLI args from a HarnessProfile
  - Files: `crates/assay-harness/src/opencode.rs`, `crates/assay-harness/src/lib.rs`, `crates/assay-harness/src/snapshots/` (new snapshot files)
  - Do: Create `opencode.rs` mirroring `claude.rs` structure: `OpenCodeConfig` struct, `generate_config()`, `write_config()`, `build_cli_args()`. Map settings to OpenCode JSON format (`model`, `tools`, `permission`, `agent.steps`). Include `$schema` field via `#[serde(rename = "$schema")]`. Map hooks to AGENTS.md advisory text. Register module in `lib.rs`. Write tests: 3 profile snapshots × config artifacts, 3 write tests, 3 CLI arg tests.
  - Verify: `cargo test -p assay-harness -- opencode` passes; `cargo clippy -p assay-harness` clean
  - Done when: ~15 OpenCode tests pass including insta snapshots, write_config tempfile tests, and CLI arg tests

- [x] **T03: Cross-adapter consistency and just ready** `est:20m`
  - Why: Ensures all three adapters are consistent, no regressions, and the full workspace is clean
  - Files: `crates/assay-harness/src/codex.rs`, `crates/assay-harness/src/opencode.rs` (minor fixes if needed)
  - Do: Run `just ready` and fix any issues. Verify all three adapters produce AGENTS.md from the same `build_prompt()` call. Verify snapshot counts match expectations (~12 Claude + ~12 Codex + ~12 OpenCode). Ensure deny-unknown-fields and serde conventions are consistent. Run `cargo test -p assay-harness` to confirm ~55+ total tests.
  - Verify: `just ready` passes clean
  - Done when: `just ready` green, all snapshot files committed, total harness test count ≥ 50

## Files Likely Touched

- `crates/assay-harness/Cargo.toml`
- `crates/assay-harness/src/lib.rs`
- `crates/assay-harness/src/codex.rs` (new)
- `crates/assay-harness/src/opencode.rs` (new)
- `crates/assay-harness/src/snapshots/` (new snapshot files)
