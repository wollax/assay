---
estimated_steps: 5
estimated_files: 5
---

# T01: Codex adapter with snapshot tests

**Slice:** S04 — Codex & OpenCode Adapters
**Milestone:** M002

## Description

Implement the Codex adapter following the exact same pattern as `claude.rs`. The adapter translates a `HarnessProfile` into Codex-specific config artifacts: `AGENTS.md` (via shared `build_prompt()`), `.codex/config.toml` (TOML settings), and CLI args for `codex exec`. Hooks are mapped to advisory AGENTS.md text since Codex lacks a native hook mechanism. All functions are pure — no I/O in `generate_config()`, filesystem writes isolated in `write_config()`.

## Steps

1. Add `toml.workspace = true` to `crates/assay-harness/Cargo.toml` dependencies section.
2. Create `crates/assay-harness/src/codex.rs` with:
   - `CodexConfig` struct: `agents_md: String`, `config_toml: String`, `model: Option<String>`
   - `generate_config(profile: &HarnessProfile) -> CodexConfig`:
     - Build `agents_md` via `build_prompt(&profile.prompt_layers)`, appending hook advisory text if hooks present
     - Build `config_toml` by serializing a `CodexConfigToml` serde struct with: `model`, `approval_policy = "full-auto"`, `sandbox_mode` (mapped from permissions — default `"workspace-write"`, escalate to `"danger-full-access"` only for network/system ops)
   - `write_config(config: &CodexConfig, dir: &Path) -> io::Result<()>`:
     - Write `AGENTS.md` (skip if empty), `.codex/config.toml` (create `.codex/` dir)
   - `build_cli_args(config: &CodexConfig) -> Vec<String>`:
     - `["exec", "--full-auto"]`, optional `--sandbox`, optional `--model`
3. Add `pub mod codex;` to `crates/assay-harness/src/lib.rs`.
4. Write test module in `codex.rs` with:
   - `realistic_profile` — full profile with prompt layers, settings, hooks → 3 snapshots (agents_md, config_toml, model assertion)
   - `minimal_profile` — empty everything → 2 snapshots (agents_md, config_toml)
   - `hooks_no_model` — hooks present, no model → 2 snapshots (agents_md showing advisory text, config_toml)
   - `hooks_advisory_in_agents_md` — verify hooks text appears in agents_md when hooks non-empty
   - `write_config_full` — tempfile test: writes AGENTS.md, .codex/config.toml
   - `write_config_creates_codex_dir` — tempfile test: .codex/ created automatically
   - `write_config_skips_empty_agents_md` — tempfile test: no AGENTS.md when empty
   - `build_cli_args_full` — snapshot of full args
   - `build_cli_args_no_model` — snapshot without model
   - `build_cli_args_minimal` — minimal config args
5. Run `cargo test -p assay-harness -- codex` and `cargo insta review` to accept snapshots.

## Must-Haves

- [ ] `CodexConfig` struct with pre-serialized strings
- [ ] `generate_config()` is a pure function (no I/O)
- [ ] `.codex/config.toml` uses proper TOML serialization via `toml` crate (not hand-built strings)
- [ ] Hooks mapped to advisory AGENTS.md text (not silently dropped)
- [ ] `sandbox_mode` defaults to `"workspace-write"` when permissions are non-empty
- [ ] `build_cli_args()` produces `codex exec --full-auto` format
- [ ] All tests pass including insta snapshot acceptance

## Verification

- `cargo test -p assay-harness -- codex` — all ~12 tests pass
- `cargo clippy -p assay-harness -- -D warnings` — no warnings
- Snapshot files exist in `crates/assay-harness/src/snapshots/` with `codex` prefix

## Observability Impact

- Signals added/changed: None — pure functions
- How a future agent inspects this: Read snapshot files in `src/snapshots/` to see exact expected Codex config format
- Failure state exposed: Insta snapshot diffs show exact mismatch on any regression

## Inputs

- `crates/assay-harness/src/claude.rs` — reference adapter pattern to mirror
- `crates/assay-types/src/harness.rs` — HarnessProfile, SettingsOverride, HookContract types
- `crates/assay-harness/src/prompt.rs` — `build_prompt()` for AGENTS.md assembly
- S04 research: Codex config format (TOML, `config.toml`, `AGENTS.md`, `codex exec` flags)

## Expected Output

- `crates/assay-harness/Cargo.toml` — `toml.workspace = true` added
- `crates/assay-harness/src/codex.rs` — complete Codex adapter (~200 lines + ~200 lines tests)
- `crates/assay-harness/src/lib.rs` — `pub mod codex;` added
- `crates/assay-harness/src/snapshots/` — ~8 new Codex snapshot files
