# S04: Codex & OpenCode Adapters — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: Adapters are pure functions (no runtime, no I/O beyond file writes). Insta snapshots lock the exact output format. There is no live agent interaction to test — runtime integration is S05/S06 scope.

## Preconditions

- Rust toolchain installed with `cargo`
- Working directory is the assay project root
- `just` command runner available

## Smoke Test

Run `cargo test -p assay-harness` and confirm 49 tests pass with 0 failures.

## Test Cases

### 1. Codex adapter generates valid TOML config

1. Run `cargo test -p assay-harness -- codex::tests::realistic_profile`
2. **Expected:** Test passes. Inspect `crates/assay-harness/src/snapshots/assay_harness__codex__tests__codex_realistic_config_toml.snap` — contains valid TOML with `[model]`, `approval_policy = "full-auto"`, and `sandbox_mode`.

### 2. OpenCode adapter generates valid JSON config

1. Run `cargo test -p assay-harness -- opencode::tests::realistic_profile`
2. **Expected:** Test passes. Inspect `crates/assay-harness/src/snapshots/assay_harness__opencode__tests__opencode_realistic_config_json.snap` — contains valid JSON with `$schema` field, model, tools map, and permissions map.

### 3. Codex AGENTS.md includes hook advisory

1. Run `cargo test -p assay-harness -- codex::tests::hooks_no_model`
2. **Expected:** Test passes. Inspect `crates/assay-harness/src/snapshots/assay_harness__codex__tests__codex_hooks_no_model_agents_md.snap` — contains "## Lifecycle Hooks (Advisory)" section with event labels and timeout info.

### 4. OpenCode AGENTS.md includes hook advisory

1. Run `cargo test -p assay-harness -- opencode::tests::hooks_no_model`
2. **Expected:** Test passes. Inspect `crates/assay-harness/src/snapshots/assay_harness__opencode__tests__opencode_hooks_no_model_agents_md.snap` — contains "## Lifecycle Hooks (Advisory)" section.

### 5. All three adapters share build_prompt()

1. Run `cargo test -p assay-harness`
2. Compare AGENTS.md snapshots across Claude/Codex/OpenCode for the same profile — prompt content sections should be identical (differences only in hook advisory formatting and adapter-specific text).
3. **Expected:** Prompt layers (project conventions, spec criteria) are consistent across all three adapters.

### 6. Write config creates correct files

1. Run `cargo test -p assay-harness -- codex::tests::write_config_creates_codex_dir`
2. Run `cargo test -p assay-harness -- opencode::tests::write_config_creates_files`
3. **Expected:** Both pass. Codex creates `.codex/config.toml` and `AGENTS.md`. OpenCode creates `opencode.json` and `AGENTS.md`.

### 7. CLI args are correct

1. Run `cargo test -p assay-harness -- codex::tests::build_cli_args_full`
2. Run `cargo test -p assay-harness -- opencode::tests::build_cli_args_full`
3. **Expected:** Codex produces `["exec", "--full-auto", "--model", "<model>"]`. OpenCode produces `["opencode", "run", "--model", "<model>", "--format", "json"]`.

## Edge Cases

### Empty/minimal profile

1. Run `cargo test -p assay-harness -- codex::tests::minimal_profile`
2. Run `cargo test -p assay-harness -- opencode::tests::minimal_profile`
3. **Expected:** Both pass. Minimal profiles produce valid config with sensible defaults — no panics, no empty required fields.

### Codex sandbox escalation

1. Run `cargo test -p assay-harness -- codex::tests::sandbox_escalation_network`
2. **Expected:** Passes. Network/system permissions escalate sandbox_mode to `danger-full-access`.

### OpenCode $schema field always present

1. Run `cargo test -p assay-harness -- opencode::tests::schema_field_present`
2. **Expected:** Passes. Even minimal config includes `$schema` field in JSON output.

## Failure Signals

- Any snapshot mismatch in `cargo test -p assay-harness` — indicates adapter output has drifted
- `just ready` failure on lint or deny — indicates code quality regression
- Missing snapshot files in `crates/assay-harness/src/snapshots/` — indicates incomplete test coverage

## Requirements Proved By This UAT

- R024 — All three harness adapters (Claude, Codex, OpenCode) generate valid config from the same HarnessProfile input, locked by 30 insta snapshots and structural assertions.
- R009 — Zero traits: all adapter functions are plain functions, not trait methods. Pattern holds across all three adapters.

## Not Proven By This UAT

- Runtime invocation of Codex or OpenCode agents with generated config — requires real agent binaries (manual UAT)
- CLI dispatch (`assay harness generate codex|opencode`) — wired in S05
- Orchestrator integration (multi-agent config generation) — wired in S06
- Config format validation against official Codex/OpenCode schemas — no official schemas available for validation

## Notes for Tester

- All tests are deterministic and reproducible — no network, no randomness, no time sensitivity.
- Snapshot files are the ground truth. If you want to see what the adapters produce, read the `.snap` files directly.
- The `toml` crate serialization uses alphabetical key ordering — this is deterministic but may differ from hand-written TOML.
