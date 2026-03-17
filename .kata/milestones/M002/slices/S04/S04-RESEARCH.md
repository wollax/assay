# S04: Codex & OpenCode Adapters — Research

**Date:** 2026-03-17

## Summary

S04 adds two new harness adapters (Codex and OpenCode) following the exact pattern established by the Claude Code adapter in M001/S04. Both target agents are installed locally (`codex` v0.114.0, `opencode` v1.2.20), and their config formats have been fully characterized through CLI help output, installed config files, and source inspection.

The Claude adapter pattern is clean and directly extensible: `generate_config()` (pure function, HarnessProfile → Config struct), `write_config()` (writes files to a directory), and `build_cli_args()` (produces CLI argument vector). Each new adapter follows this same three-function signature with adapter-specific config types. The work is straightforward — no new architectural decisions, no dependency changes, no cross-crate modifications needed.

The primary recommendation is: **implement `codex.rs` and `opencode.rs` modules in `assay-harness`** mirroring `claude.rs` exactly. Each module produces its target agent's config format from the shared `HarnessProfile` input. Verified by insta snapshot tests (same pattern as Claude's 12 snapshots) and structural assertions.

## Recommendation

Add two new modules to `assay-harness/src/`: `codex.rs` and `opencode.rs`. Each follows the Claude adapter's three-function pattern:

1. **`generate_config(profile: &HarnessProfile) -> XConfig`** — Pure function mapping HarnessProfile to agent-specific config. No I/O.
2. **`write_config(config: &XConfig, dir: &Path) -> io::Result<()>`** — Writes config files to the worktree directory.
3. **`build_cli_args(config: &XConfig) -> Vec<String>`** — Produces CLI argument vector for non-interactive execution.

Register both modules in `lib.rs`. No changes to `assay-types`, `assay-core`, or any other crate needed. The existing `HarnessProfile` type (already proven) is the shared input contract.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| TOML serialization (Codex config) | `toml` crate (already in workspace deps via manifest parsing) | Codex config.toml is TOML format; reuse existing dep |
| JSON serialization (OpenCode config) | `serde_json` (already a dep of assay-harness) | OpenCode opencode.json is JSON; already available |
| Snapshot testing | `insta` (already a dev-dep of assay-harness) | Proven pattern from Claude adapter's 12 snapshots |
| Prompt assembly | `crate::prompt::build_prompt()` | Shared across all adapters — assembles PromptLayers into markdown |

## Existing Code and Patterns

- `crates/assay-harness/src/claude.rs` — **Primary reference pattern.** Three public functions: `generate_config()`, `write_config()`, `build_cli_args()`. Config struct (`ClaudeConfig`) holds pre-serialized strings. 27 tests with 12 insta snapshots. Copy this structure exactly for both new adapters.
- `crates/assay-harness/src/prompt.rs` — `build_prompt(&[PromptLayer]) -> String` — shared prompt builder used by all adapters. Assembles layers by priority into `## Name\n\ncontent` sections separated by `---`. Reuse directly.
- `crates/assay-harness/src/settings.rs` — `merge_settings()` for layered settings override. May be useful if adapters need to merge base + override settings.
- `crates/assay-types/src/harness.rs` — `HarnessProfile`, `SettingsOverride`, `HookContract`, `HookEvent`, `PromptLayer`, `PromptLayerKind` — all input types. Stable, locked by schema snapshots.
- `crates/assay-core/src/pipeline.rs` — `HarnessWriter = dyn Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String>` — the closure type that S05/S06 will use to dispatch to the correct adapter. Each adapter's `write_config` + `build_cli_args` composes into this signature.
- `plugins/codex/AGENTS.md` — Existing project-level Codex plugin with instructions template. Shows the expected output format for Codex's AGENTS.md file.
- `plugins/opencode/opencode.json` — Existing project-level OpenCode plugin skeleton. Shows the expected config structure.

## Target Agent Config Formats

### Codex CLI

**Global config:** `~/.codex/config.toml` (TOML)
```toml
model = "gpt-5.3-codex"
model_reasoning_effort = "high"
project_doc_fallback_filenames = ["CLAUDE.md", "AGENTS.md"]

[projects."/path/to/project"]
trust_level = "trusted"
```

**Project instructions:** `AGENTS.md` at project root (read via `project_doc_fallback_filenames`). Same markdown format as Claude's `CLAUDE.md`.

**Agent definitions:** `~/.codex/agents/<name>.toml`
```toml
sandbox_mode = "workspace-write"
developer_instructions = """
...multi-line instructions...
"""
```

**Non-interactive execution:** `codex exec --model MODEL --sandbox MODE -C DIR "PROMPT"`
- `--model` — model identifier
- `--sandbox` — `read-only | workspace-write | danger-full-access`
- `--full-auto` — convenience for `-a on-request --sandbox workspace-write`
- `-C DIR` — working directory
- `--json` — JSONL event output
- `--output-last-message FILE` — write last agent message to file
- `--ephemeral` — no session persistence

**Adapter output files:**
1. `AGENTS.md` — assembled prompt (same as Claude's CLAUDE.md)
2. (No separate settings file — Codex reads config.toml for settings, or uses CLI args)

**Config struct fields:**
- `agents_md: String` — assembled prompt from layers
- `model: Option<String>` — model override for CLI args
- `sandbox_mode: String` — sandbox policy (map from HarnessProfile permissions)
- `full_auto: bool` — whether to use --full-auto mode

### OpenCode

**Global config:** `~/.config/opencode/opencode.json` (JSON)
```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": ["oh-my-opencode"],
  "permission": {
    "read": { "pattern": "action" },
    "external_directory": { "pattern": "action" }
  }
}
```

**Project instructions:** `AGENTS.md` at project root (also reads `CLAUDE.md` and deprecated `CONTEXT.md`). Same markdown format. OpenCode reads these via `InstructionPrompt` namespace that searches upward from project root.

**Agent definitions:** `~/.config/opencode/agents/<name>.md` with YAML frontmatter:
```markdown
---
description: Agent description
color: "#FFFF00"
tools:
  read: true
  write: true
  edit: true
  bash: true
  grep: true
  glob: true
---

Instructions content here...
```

**Non-interactive execution:** `opencode run --model PROVIDER/MODEL --agent AGENT --dir DIR --format json "MESSAGE"`
- `--model` — `provider/model` format
- `--agent` — agent name to use
- `--dir` — working directory
- `--format json` — raw JSON event output
- `--command` — command to run

**Permission model:** Object with permission types (`read`, `write`, `edit`, `bash`, `grep`, `glob`, `external_directory`, `doom_loop`, `question`, `plan_enter`, `plan_exit`) mapped to patterns with `allow`/`ask`/`deny` actions.

**Adapter output files:**
1. `AGENTS.md` — assembled prompt (same format as Claude/Codex)
2. `opencode.json` — project-level config with permissions

**Config struct fields:**
- `agents_md: String` — assembled prompt from layers
- `opencode_json: String` — project config JSON with permissions
- `model: Option<String>` — model in `provider/model` format
- `agent: Option<String>` — agent name for CLI args

## Constraints

- **Zero traits (D001):** New adapters are plain functions, not trait implementations. Same pattern as `claude.rs`.
- **Harness crate only (D003):** All adapter code lives in `assay-harness`. No changes to `assay-core` or `assay-types`.
- **Shared HarnessProfile contract (D010):** Both adapters consume the same `HarnessProfile` type. No adapter-specific input types.
- **Closures for pipeline integration (D015):** Each adapter's `write_config` + `build_cli_args` will compose into a `HarnessWriter` closure at the call site (S05/S06 concern, not S04).
- **`toml` crate needed for Codex:** Codex config is TOML. Check if `toml` is already a workspace dep (used for manifest parsing in assay-types). If so, add to assay-harness deps. If not, add as workspace dep.
- **No hooks in Codex/OpenCode:** Unlike Claude Code which has a rich hooks system (PreToolUse, PostToolUse, Stop), Codex and OpenCode have different hook models. OpenCode has tool-level permissions and optional hooks in agent frontmatter. Codex has no equivalent hooks format. Map `HookContract` entries to inline instructions in the prompt where the target agent lacks native hook support.

## Common Pitfalls

- **Codex sandbox mode mapping** — `HarnessProfile.settings.permissions` is a `Vec<String>` of permission names (e.g., `"Bash(*)"`, `"Read(*)"`, `"Write(*)"`). These need meaningful mapping to Codex's three sandbox modes: `read-only` (no write perms), `workspace-write` (write perms present), `danger-full-access` (explicit). Default to `workspace-write` when write permissions are present, `read-only` otherwise. Don't default to `danger-full-access`.
- **OpenCode permission format** — OpenCode permissions use a structured object with pattern matching, not a simple string list. Need to map HarnessProfile permissions to OpenCode's `{ "permission_type": { "pattern": "action" } }` format. Keep it simple: map to broad `"*": "allow"` patterns based on what permissions are present.
- **Instructions file naming** — Claude uses `CLAUDE.md`, Codex and OpenCode both read `AGENTS.md`. The adapter should write `AGENTS.md` (not `CLAUDE.md`) for Codex and OpenCode worktrees.
- **Hooks degradation** — If `HookContract` entries exist but the target agent doesn't support native hooks, embed them as instructions in the prompt ("Before using any tool, run: ..."). Don't silently drop them. Log or document the degradation.
- **Model format difference** — Claude uses bare model names (`sonnet`, `opus`). Codex uses bare names (`o3`). OpenCode uses `provider/model` format (`anthropic/claude-sonnet-4`). The `model` field in `SettingsOverride` is a plain string — adapters may need to interpret or pass-through differently.
- **Empty config edge cases** — Minimal profiles (no prompt layers, no hooks, empty permissions) should produce valid but minimal config. Test this explicitly (the Claude adapter already does via `minimal_profile` test).

## Open Risks

- **Codex config.toml project trust** — Codex requires `trust_level = "trusted"` per-project in `~/.codex/config.toml` for auto-execution. The adapter generates worktree-level config but cannot modify the global config. The orchestrator may need to handle this at a higher level (S05/S06 concern). For S04, document but don't solve — `write_config` writes to the worktree only.
- **OpenCode plugin system** — OpenCode has a plugin system (`"plugin": ["oh-my-opencode"]`). The adapter should not interfere with existing plugins. Generated `opencode.json` should only set permissions, not override the user's plugin config. Consider generating a minimal config that can be merged rather than a complete replacement.
- **Config format evolution** — Both Codex and OpenCode are actively developed and their config formats may change. Snapshot tests lock the current expected output but won't detect if the target agent's parser changes. This is acceptable risk — config formats are typically backward-compatible.
- **No native hooks support in either agent** — Neither Codex nor OpenCode has Claude Code's hooks.json format. HookContract entries must degrade to prompt instructions. This is a fidelity loss but acceptable for S04 scope.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust general | (available in `<available_skills>`) | none directly relevant |
| Codex CLI | none found | n/a — config format is simple TOML, no skill needed |
| OpenCode | none found | n/a — config format is simple JSON, no skill needed |

No skills are relevant to install. The work is straightforward adapter code using `serde_json` and potentially `toml` for serialization, following the proven Claude adapter pattern.

## Sources

- **Codex CLI config format**: `~/.codex/config.toml` (locally installed v0.114.0), `~/.codex/agents/*.toml` agent definition format, `codex exec --help` CLI interface
- **OpenCode config format**: `~/.config/opencode/opencode.json` (locally installed v1.2.20), `~/.config/opencode/agents/*.md` agent definition format with YAML frontmatter, `opencode run --help` CLI interface
- **OpenCode instruction files**: Source inspection of opencode-darwin-arm64/bin/index.js.map — confirmed `InstructionPrompt` namespace reads `["AGENTS.md", "CLAUDE.md", "CONTEXT.md"]` from project root, plus global `AGENTS.md` from config dir and `~/.claude/CLAUDE.md`
- **Existing Claude adapter**: `crates/assay-harness/src/claude.rs` — proven three-function pattern with 27 tests
- **Existing plugins**: `plugins/codex/AGENTS.md` and `plugins/opencode/opencode.json` — confirm expected output formats for each target agent
