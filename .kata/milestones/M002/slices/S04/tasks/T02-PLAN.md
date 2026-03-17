---
estimated_steps: 4
estimated_files: 3
---

# T02: OpenCode adapter with snapshot tests

**Slice:** S04 — Codex & OpenCode Adapters
**Milestone:** M002

## Description

Implement the OpenCode adapter following the same pattern as `claude.rs` and the newly completed `codex.rs`. The adapter translates a `HarnessProfile` into OpenCode-specific config artifacts: `AGENTS.md` (via shared `build_prompt()`), `opencode.json` (JSON settings with `$schema`), and CLI args for `opencode run`. Hooks are mapped to advisory AGENTS.md text since OpenCode lacks a native hook mechanism.

## Steps

1. Create `crates/assay-harness/src/opencode.rs` with:
   - `OpenCodeConfig` struct: `agents_md: String`, `config_json: String`, `model: Option<String>`
   - Internal `OpenCodeConfigJson` serde struct for serialization with:
     - `#[serde(rename = "$schema")] schema: String` (set to `"https://opencode.ai/config.json"`)
     - `model: Option<String>` — `"provider/model-id"` pass-through
     - `tools: BTreeMap<String, bool>` — enabled tools from `SettingsOverride.tools`
     - `permission: BTreeMap<String, String>` — `"allow"` for each tool in permissions
     - `agent: Option<AgentConfig>` with `steps: Option<u32>` from `max_turns`
   - `generate_config(profile: &HarnessProfile) -> OpenCodeConfig`:
     - Build `agents_md` via `build_prompt()`, appending hook advisory text if hooks present (same pattern as Codex)
     - Build `config_json` by serializing `OpenCodeConfigJson` via `serde_json::to_string_pretty()`
   - `write_config(config: &OpenCodeConfig, dir: &Path) -> io::Result<()>`:
     - Write `AGENTS.md` (skip if empty), `opencode.json`
   - `build_cli_args(config: &OpenCodeConfig) -> Vec<String>`:
     - `["run"]`, optional `--model`, `--format json`
2. Add `pub mod opencode;` to `crates/assay-harness/src/lib.rs`.
3. Write test module in `opencode.rs` with:
   - `realistic_profile` — full profile → 2 snapshots (agents_md, config_json) + model assertion
   - `minimal_profile` — empty everything → 2 snapshots
   - `hooks_no_model` — hooks present → 2 snapshots (agents_md with advisory, config_json)
   - `schema_field_present` — programmatic assertion that config_json contains `$schema`
   - `write_config_full` — tempfile: AGENTS.md + opencode.json written
   - `write_config_creates_files` — tempfile: both files exist
   - `write_config_skips_empty_agents_md` — tempfile: no AGENTS.md when empty
   - `build_cli_args_full` — snapshot of full args
   - `build_cli_args_no_model` — snapshot without model
   - `build_cli_args_minimal` — minimal args
4. Run `cargo test -p assay-harness -- opencode` and `cargo insta review` to accept snapshots.

## Must-Haves

- [ ] `OpenCodeConfig` struct with pre-serialized strings
- [ ] `generate_config()` is a pure function (no I/O)
- [ ] `opencode.json` includes `$schema` field via serde rename
- [ ] Tools mapped from `SettingsOverride.tools` to `{"tool_name": true}` format
- [ ] Permissions mapped to `{"edit": "allow", "bash": "allow"}` format
- [ ] `max_turns` mapped to agent steps config
- [ ] Hooks mapped to advisory AGENTS.md text (same pattern as Codex)
- [ ] `build_cli_args()` produces `opencode run` format with `--format json`
- [ ] All tests pass including insta snapshot acceptance

## Verification

- `cargo test -p assay-harness -- opencode` — all ~12 tests pass
- `cargo clippy -p assay-harness -- -D warnings` — no warnings
- Snapshot files exist in `crates/assay-harness/src/snapshots/` with `opencode` prefix

## Observability Impact

- Signals added/changed: None — pure functions
- How a future agent inspects this: Read snapshot files to see exact expected OpenCode config format
- Failure state exposed: Insta snapshot diffs show exact mismatch on any regression

## Inputs

- `crates/assay-harness/src/claude.rs` — reference adapter pattern
- `crates/assay-harness/src/codex.rs` — sibling adapter from T01 (same hook advisory pattern)
- `crates/assay-types/src/harness.rs` — input types
- `crates/assay-harness/src/prompt.rs` — `build_prompt()` for AGENTS.md
- S04 research: OpenCode config format (JSON, `opencode.json`, `opencode run` flags)

## Expected Output

- `crates/assay-harness/src/opencode.rs` — complete OpenCode adapter (~200 lines + ~200 lines tests)
- `crates/assay-harness/src/lib.rs` — `pub mod opencode;` added
- `crates/assay-harness/src/snapshots/` — ~8 new OpenCode snapshot files
