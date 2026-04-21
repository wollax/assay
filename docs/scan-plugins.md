# Assay Plugins — Comprehensive Scan

Scan date: 2026-04-21
Source: `/Users/wollax/Git/personal/assay/plugins/`

---

## Plugin Inventory

| Plugin | Target Platform | Files | Purpose |
| --- | --- | --- | --- |
| **claude-code** | Claude Code (Anthropic) | plugin.json, .mcp.json, CLAUDE.md, hooks.json, 4 shell scripts, 9 skill SKILL.md files, README.md | Full-featured plugin with MCP server registration, hook-based gate enforcement, PostToolUse reminders, checkpoint persistence, and 9 skills for spec-driven development |
| **codex** | OpenAI Codex | AGENTS.md, README.md, 9 skill .md files | Lightweight skill-only plugin; no hooks, no manifest, no MCP config. Skills are symlinked into `.agents/skills/assay/` |
| **opencode** | OpenCode | AGENTS.md, README.md, opencode.json, package.json, tsconfig.json, 9 skill .md files, placeholder dirs (commands/, plugins/, tools/) | NPM-style plugin package (`@assay/opencode-plugin`). Includes MCP tool table in AGENTS.md but no explicit MCP config file. Has TypeScript scaffolding (tsconfig.json) but no source files yet |
| **smelt-agent** | Smelt worker agents | AGENTS.md, 4 skill .md files, tests/verify-docs.sh | Infrastructure-focused plugin for orchestrated run dispatch, multi-machine peer discovery, signal forwarding, and mesh/gossip inter-session communication |

### File Counts

| Plugin | Config Files | Skills | Scripts/Hooks | Tests | Docs |
| --- | --- | --- | --- | --- | --- |
| claude-code | 3 (plugin.json, .mcp.json, hooks.json) | 9 | 4 shell scripts | 0 | 2 (README.md, CLAUDE.md) |
| codex | 0 | 9 | 0 | 0 | 2 (README.md, AGENTS.md) |
| opencode | 3 (opencode.json, package.json, tsconfig.json) | 9 | 0 | 0 | 2 (README.md, AGENTS.md) |
| smelt-agent | 0 | 4 | 0 | 1 (verify-docs.sh) | 1 (AGENTS.md) |

---

## Plugin Architecture

### claude-code

The most complete plugin. Uses Claude Code's native plugin system:

- **`.claude-plugin/plugin.json`** — Plugin manifest with name (`assay`), version (`0.5.0`), description, author, homepage, and license.
- **`.mcp.json`** — Registers the `assay` MCP server as a stdio process (`assay mcp serve`).
- **`hooks/hooks.json`** — Declares PostToolUse, PreCompact, and Stop hooks with matchers and shell command handlers.
- **`scripts/`** — Four bash scripts implementing hook behavior (post-tool-use reminders, checkpoint saves, stop-time gate enforcement).
- **`skills/*/SKILL.md`** — Each skill is a directory containing a `SKILL.md` file with YAML frontmatter (`name`, `description`) and markdown body with step-by-step instructions.
- **`CLAUDE.md`** — Agent-facing instructions injected into the conversation context.
- **`agents/`** and **`commands/`** — Empty placeholder directories (`.gitkeep`).

### codex

Minimal structure. No manifest, no MCP configuration, no hooks:

- **`AGENTS.md`** — Agent instructions (equivalent to CLAUDE.md for Codex).
- **`skills/*.md`** — Flat markdown files (no subdirectories). Some have YAML frontmatter (`plan.md`, `spec-show.md`), others are plain markdown.
- Installation is via symlink: `ln -s /path/to/plugins/codex/skills .agents/skills/assay`

### opencode

NPM package structure with TypeScript scaffolding:

- **`opencode.json`** — OpenCode-specific plugin descriptor (`@assay/opencode-plugin`, version `0.1.0`).
- **`package.json`** — NPM package with `"type": "module"`, `"private": true`.
- **`tsconfig.json`** — TypeScript config targeting ES2022 with bundler resolution. No source files exist yet.
- **`AGENTS.md`** — Agent instructions (identical content to codex's AGENTS.md).
- **`skills/*.md`** — Flat markdown files, same format as codex.
- **Placeholder directories** — `commands/`, `plugins/`, `tools/` (all `.gitkeep`).

### smelt-agent

Documentation-only plugin for worker agents in orchestrated runs:

- **`AGENTS.md`** — Detailed agent instructions covering run dispatch, backend configuration, peer messaging, and signal forwarding.
- **`skills/*.md`** — Four skills with YAML frontmatter, covering infrastructure concerns (run-dispatch, backend-status, peer-message, peer-registry).
- **`tests/verify-docs.sh`** — Structural test that validates all MCP tool names referenced in docs exist in `assay-mcp/src/server.rs`.

### Manifest/Config Format Summary

| Format | Used By | Purpose |
| --- | --- | --- |
| `.claude-plugin/plugin.json` | claude-code | Claude Code plugin manifest (name, version, description, author, license) |
| `.mcp.json` | claude-code | MCP server registration (stdio transport) |
| `hooks/hooks.json` | claude-code | Hook declarations with matchers and command handlers |
| `opencode.json` | opencode | OpenCode plugin descriptor |
| `package.json` | opencode | NPM package metadata |
| `tsconfig.json` | opencode | TypeScript compilation config |
| `CLAUDE.md` | claude-code | Agent instructions for Claude Code |
| `AGENTS.md` | codex, opencode, smelt-agent | Agent instructions for non-Claude platforms |

---

## MCP Integration

### How MCP is registered

Only **claude-code** has an explicit MCP configuration file (`.mcp.json`):

```json
{
  "mcpServers": {
    "assay": {
      "type": "stdio",
      "command": "assay",
      "args": ["mcp", "serve"]
    }
  }
}
```

This launches the `assay` binary with `mcp serve` subcommand as a stdio MCP server. The binary must be on PATH (installed via `cargo install assay-cli` or built from source).

### MCP Tools Available

All plugins reference the same set of MCP tools (exposed by `assay mcp serve`). The shared tools are:

| Tool | Description | Used By |
| --- | --- | --- |
| `spec_list` | List all specs in the project | All 4 plugins |
| `spec_get` | Get a spec's full definition and criteria | All 4 plugins |
| `spec_create` | Create a spec for a chunk with criteria | claude-code, codex, opencode |
| `gate_run` | Run quality gates for a spec | All 4 plugins |
| `cycle_status` | Get active milestone progress | All 4 plugins |
| `cycle_advance` | Advance the active chunk | All 4 plugins |
| `chunk_status` | Get gate results for a specific chunk | All 4 plugins |
| `milestone_list` | List all milestones | All 4 plugins |
| `milestone_get` | Get a milestone by slug | claude-code only |
| `milestone_create` | Create a new milestone with chunks | claude-code, codex, opencode |
| `pr_create` | Create a gate-gated PR | claude-code, codex, opencode |

**Smelt-agent additional tools:**

| Tool | Description |
| --- | --- |
| `run_manifest` | Execute a single-session manifest |
| `orchestrate_run` | Launch a multi-session orchestrated run |
| `orchestrate_status` | Query status of an orchestrated run by run_id |
| `poll_signals` | Read PeerUpdate messages from a session's signal inbox |
| `send_signal` | POST a SignalRequest to any signal endpoint URL |
| `merge_propose` | Push branch and create a GitHub PR with gate evidence |

### How codex and opencode access MCP

Neither codex nor opencode have their own `.mcp.json`. They document the tool table in `AGENTS.md` and rely on the host platform's MCP configuration to register the `assay` MCP server externally. The plugin's role for these platforms is limited to providing skill definitions and agent instructions.

---

## Skills & Commands

### claude-code Skills (9 total)

| Skill | Invocation | Status | Description |
| --- | --- | --- | --- |
| **explore** | `/assay:explore` | Active | Load lean project context (~500 tokens) for brainstorming and exploration. Tiered loading: always loads cycle_status + spec_list; conditionally loads chunk_status + git log. On-demand deep dives. |
| **plan** | `/assay:plan` | Active | Interview-guided milestone creation. Supports `quick` mode for flat single-chunk specs. Collects all inputs before calling MCP tools. Creates milestone + per-chunk specs. |
| **focus** | `/assay:focus` | Active | Show active work context: milestone, chunk, criteria, gate status, progress. Replaces both `status` and `next-chunk`. Hides milestone/chunk terminology for quick milestones. |
| **check** | `/assay:check [name]` | Active | Run quality gates with smart routing. Auto-detects active chunk if no name given. Reports pass/fail with next-step suggestions. Replaces `gate-check`. |
| **ship** | `/assay:ship` | Active | Verify gates pass, then create PR with gate evidence via `pr_create`. Blocks PR creation on gate failures. |
| **spec-show** | `/assay:spec-show [name]` | Active | Display a spec's full definition. Groups criteria by type (executable vs descriptive). Shows commands and timeouts. |
| **gate-check** | `/assay:gate-check` | Deprecated | Redirects to `/assay:check`. Same content as check skill with deprecation notice. |
| **next-chunk** | `/assay:next-chunk` | Deprecated | Redirects to `/assay:focus`. Same content as focus skill with deprecation notice. |
| **status** | `/assay:status` | Deprecated | Redirects to `/assay:focus`. Same content as focus skill with deprecation notice. |

### codex Skills (9 total)

| Skill | File | Status | Description |
| --- | --- | --- | --- |
| **explore** | `explore.md` | Active | Identical to claude-code explore (no frontmatter) |
| **plan** | `plan.md` | Active | Extended version with 6-step interview (includes slug collision check + manual cmd warning). Has YAML frontmatter. |
| **focus** | `focus.md` | Active | Identical to claude-code focus (no frontmatter) |
| **check** | `check.md` | Active | Identical to claude-code check (no frontmatter) |
| **ship** | `ship.md` | Active | Identical to claude-code ship (no frontmatter) |
| **spec-show** | `spec-show.md` | Active | Identical to claude-code spec-show. Has YAML frontmatter. |
| **gate-check** | `gate-check.md` | Deprecated | Redirects to check |
| **next-chunk** | `next-chunk.md` | Deprecated | Redirects to focus |
| **cycle-status** | `cycle-status.md` | Deprecated | Redirects to focus |

### opencode Skills (9 total)

Identical set and content to codex skills. Same 9 files with the same markdown content.

### smelt-agent Skills (4 total)

| Skill | Invocation | Description |
| --- | --- | --- |
| **run-dispatch** | `/assay:run-dispatch` | Read a RunManifest TOML, optionally configure StateBackendConfig, choose `run_manifest` (single) or `orchestrate_run` (multi-session), and dispatch. Documents all 6 backend types: local_fs, linear, github, ssh, smelt, custom. |
| **backend-status** | `/assay:backend-status` | Query `orchestrate_status` and interpret the OrchestratorStatus response. Covers run phases (running/completed/partial_failure/aborted), per-session states, mesh_status, gossip_status, and CapabilitySet degradation. |
| **peer-message** | `/assay:peer-message` | Inter-session communication. Mesh mode: direct messaging via outbox/inbox file routing. Gossip mode: read shared knowledge manifest. Signal mode: HTTP-based `poll_signals` and `send_signal` for cross-instance communication. |
| **peer-registry** | `/assay:peer-registry` | Multi-machine peer discovery and registration. Documents PeerInfo type, backend methods (register/list/unregister), automatic lifecycle, cross-instance signal forwarding with loop prevention, and multi-machine setup instructions. |

### CLI Commands (claude-code CLAUDE.md)

| Command | Description |
| --- | --- |
| `assay plan` | Interactive milestone wizard (CLI) |
| `assay milestone list` | List all milestones |
| `assay milestone status` | Show in-progress milestone progress |
| `assay milestone advance` | Evaluate gates and mark active chunk complete |
| `assay pr create <slug>` | Gate-gated PR creation via `gh` |

---

## Agent Instructions

### claude-code (`CLAUDE.md`)

Structured as a reference card with three sections:

1. **Skills table** — 6 active skills with invocation syntax and one-line descriptions
2. **CLI Commands table** — 5 commands for milestone management and PR creation
3. **MCP Tools table** — 11 tools with descriptions

The instructions frame the workflow as "milestone-driven spec development" and present skills as the primary interface, with CLI commands and MCP tools as supporting layers.

### codex (`AGENTS.md`)

Identical structure to opencode's AGENTS.md:

1. **Preamble** — "This project uses Assay for spec-driven development with quality gates." Explains the spec/criteria/gate model and chunk-based milestone progression.
2. **Skills table** — 6 active skills (same as claude-code)
3. **MCP Tools table** — 10 tools (omits `milestone_get`)
4. **Workflow** — 6-step numbered workflow: Plan, Read (next-chunk), Implement, Gate-check, Advance, PR

### opencode (`AGENTS.md`)

Identical content to codex AGENTS.md word-for-word. Same preamble, same tables, same workflow steps.

### smelt-agent (`AGENTS.md`)

Significantly different from the dev-facing plugins:

1. **Role framing** — "You are a smelt worker agent executing Assay runs."
2. **Skills table** — 4 infrastructure skills (run-dispatch, backend-status, peer-message, peer-registry)
3. **MCP Tools table** — 11 tools including smelt-specific ones (run_manifest, orchestrate_run, orchestrate_status, poll_signals, send_signal, merge_propose)
4. **Workflow** — 6-step run execution workflow: Receive manifest, Configure backend, Dispatch run, Monitor status, Handle messaging, Report results
5. **Backend Capability Awareness** — Detailed section on CapabilitySet flags (supports_messaging, supports_gossip_manifest, supports_annotations, supports_checkpoints, supports_signals, supports_peer_registry)
6. **Cross-Instance Signal Forwarding** — Documents the forwarding protocol and `X-Assay-Forwarded` loop prevention header
7. **Environment Variables** — ASSAY_SIGNAL_PORT, ASSAY_SIGNAL_BIND, ASSAY_SIGNAL_URL, ASSAY_SIGNAL_TOKEN

---

## Hook Configuration

Only **claude-code** has hooks. Neither codex, opencode, nor smelt-agent define any hook behavior.

### Hook Events

| Event | Matcher | Script | Timeout | Purpose |
| --- | --- | --- | --- | --- |
| **PostToolUse** | `Write\|Edit` | `post-tool-use.sh` | 5s | Reminder nudge after file edits — tells agent to run `/assay:gate-check` when ready. Includes active chunk name if a milestone is active. |
| **PostToolUse** | `Task\|TaskCreate\|TaskUpdate` | `checkpoint-hook.sh` | 5s | Save team state checkpoint on task operations |
| **PreCompact** | _(all)_ | `checkpoint-hook.sh` | 10s | Save checkpoint before context compaction |
| **Stop** | _(all)_ | `cycle-stop-check.sh` | 120s | Cycle-aware gate enforcement — blocks agent stop if quality gates fail |
| **Stop** | _(all)_ | `checkpoint-hook.sh` | 10s | Save checkpoint on stop |

### Script Details

**`post-tool-use.sh`** — Reminder-only hook. Does NOT run gates. Outputs a `hookSpecificOutput` JSON message that nudges the agent. Detects active milestone/chunk via `assay milestone status --json` and includes chunk name in the message. Gracefully degrades (no assay binary, no .assay dir, no jq).

**`checkpoint-hook.sh`** — Saves team state via `assay checkpoint save`. Features:
- 5-second debounce (writes timestamp to `.assay/checkpoints/.last-checkpoint-ts`)
- Extracts trigger info (event + tool name) and session ID from hook input
- Fire-and-forget: spawns `assay checkpoint save` in background
- Never blocks agent workflow (always exits 0)
- Guards: jq required, stop_hook_active check, .assay/ directory check, assay binary check

**`cycle-stop-check.sh`** — The primary gate enforcement hook. Extended version of `stop-gate-check.sh` with cycle awareness:
- Discovers incomplete chunks from `assay milestone status`
- Runs per-chunk gate checks (`assay gate run <chunk> --json`)
- Falls back to `assay gate run --all` when no active milestone exists
- Three modes via `ASSAY_STOP_HOOK_MODE`:
  - `enforce` (default) — blocks stop with `decision: "block"` JSON
  - `warn` — allows stop but injects `systemMessage` warning
  - `off` — disables entirely
- Names blocking chunks in the block reason for actionable diagnostics
- Captures and surfaces stderr from failed gate runs

**`stop-gate-check.sh`** — Original (non-cycle-aware) stop hook. Simpler version that always runs `gate run --all`. Same 5-guard pattern and 3-mode behavior. Appears to be superseded by `cycle-stop-check.sh` but is still present in the scripts directory (not referenced in hooks.json).

### Guard Pattern

All scripts follow a consistent 5-guard pattern (order matters):

1. `jq` not installed — allow stop (can't parse JSON)
2. `stop_hook_active = true` — allow stop (prevents infinite loops; MUST be checked before any blocking logic)
3. `ASSAY_STOP_HOOK_MODE=off` — allow stop (user disabled)
4. No `.assay/` directory — allow stop (graceful degradation)
5. `assay` binary not on PATH — allow stop (binary not installed)

---

## Configuration

### claude-code

| Setting | Source | Default | Description |
| --- | --- | --- | --- |
| `ASSAY_STOP_HOOK_MODE` | Environment variable | `enforce` | Controls stop hook behavior: `enforce` (block on gate failure), `warn` (allow but warn), `off` (disable) |
| Plugin path | `claude plugin add <path>` or `--plugin-dir` | N/A | Plugin installation path |
| `assay` binary | PATH | Required | Must be available as `assay` command |
| `jq` binary | PATH | Optional | Required for JSON output in hooks; hooks degrade gracefully without it |

### opencode

| Setting | Source | Default | Description |
| --- | --- | --- | --- |
| Plugin registration | `opencode.json` / `package.json` | N/A | Registered as `@assay/opencode-plugin` in OpenCode config |

### smelt-agent

| Setting | Source | Default | Description |
| --- | --- | --- | --- |
| `ASSAY_SIGNAL_PORT` | Environment variable | `7432` | Port for the HTTP signal listener |
| `ASSAY_SIGNAL_BIND` | Environment variable | `127.0.0.1` | Bind address (`0.0.0.0` for all interfaces / multi-machine) |
| `ASSAY_SIGNAL_URL` | Environment variable | Derived from bind+port | Override peer-registered URL; required when bind is `0.0.0.0` |
| `ASSAY_SIGNAL_TOKEN` | Environment variable | None | Optional bearer token for auth |
| `LINEAR_API_KEY` | Environment variable | None | Required for Linear backend |
| State backend type | RunManifest TOML `[state_backend]` | `local_fs` | Backend for orchestrator state persistence |

### State Backend Types (smelt-agent)

| Type | Config Fields | Status |
| --- | --- | --- |
| `local_fs` | _(none)_ | Default, fully implemented |
| `linear` | `team_id`, optional `project_id` | Stub (logs warning, falls back to no-op) |
| `github` | `repo`, optional `label` | Stub (logs warning, falls back to no-op) |
| `ssh` | `host`, `remote_assay_dir`, optional `user`/`port` | Stub (logs warning, falls back to no-op) |
| `smelt` | `url`, `job_id`, optional `token` | Fully implemented |
| `custom` | `name`, `config` | Falls back to no-op |

---

## Cross-Plugin Patterns

### Shared Patterns

1. **Skill naming convention** — All plugins use the `/assay:<skill-name>` invocation pattern. Skill names are consistent across plugins: `explore`, `plan`, `focus`, `check`, `ship`, `spec-show`.

2. **Deprecation strategy** — All three dev-facing plugins (claude-code, codex, opencode) maintain deprecated skill aliases (`gate-check` -> `check`, `next-chunk` -> `focus`, `status`/`cycle-status` -> `focus`) with deprecation notices. The old names still resolve to the new implementations.

3. **Skill content parity** — The core 6 active skills (`explore`, `plan`, `focus`, `check`, `ship`, `spec-show`) have identical logical content across all three dev-facing plugins. The codex/opencode `plan` skill has a more detailed 6-step interview (with slug collision checking and manual `cmd` warning) compared to claude-code's `plan` skill (which uses a simpler 5-step flow with `quick` mode support).

4. **MCP tool reliance** — All plugins interact with Assay exclusively through MCP tools. No plugin embeds business logic; they provide instructions for agents to call MCP tools in the correct sequence.

5. **Graceful degradation** — The hook scripts (claude-code) and capability checks (smelt-agent) both follow a pattern of failing open: missing binaries, missing directories, and unsupported features allow the workflow to continue rather than blocking.

### Differences

| Aspect | claude-code | codex | opencode | smelt-agent |
| --- | --- | --- | --- | --- |
| **MCP registration** | `.mcp.json` (explicit) | External (host config) | External (host config) | External (host config) |
| **Hook system** | 4 hooks (PostToolUse x2, PreCompact, Stop x2) | None | None | None |
| **Gate enforcement** | Automated via Stop hook | Manual (agent follows workflow) | Manual (agent follows workflow) | N/A (worker agent) |
| **Checkpoint persistence** | Automated via hooks on Write/Edit, Task, PreCompact, Stop | None | None | N/A |
| **Skill format** | `skills/<name>/SKILL.md` with YAML frontmatter | `skills/<name>.md` (mixed frontmatter) | `skills/<name>.md` (mixed frontmatter) | `skills/<name>.md` with YAML frontmatter |
| **Agent instructions file** | `CLAUDE.md` | `AGENTS.md` | `AGENTS.md` | `AGENTS.md` |
| **Agent instructions content** | Reference card (skills + CLI + MCP tools) | Role + skills + MCP + workflow steps | Identical to codex | Worker agent role + infra skills + capabilities |
| **Plugin manifest** | `plugin.json` (Claude Code format) | None | `opencode.json` + `package.json` | None |
| **Installation** | `claude plugin add` | Symlink skills dir | Register in OpenCode config | N/A (agent instructions only) |
| **TypeScript scaffolding** | No | No | Yes (tsconfig.json, no source yet) | No |
| **Test infrastructure** | No | No | No | Yes (`verify-docs.sh`) |
| **Domain focus** | Developer workflow (plan/implement/gate/ship) | Developer workflow | Developer workflow | Infrastructure (dispatch/monitor/message/forward) |

### Architecture Summary

The plugin system follows a layered approach:

- **claude-code** is the reference implementation with the richest integration (hooks, MCP config, checkpoint automation, gate enforcement).
- **codex** and **opencode** are near-identical ports that provide skill definitions and agent instructions but rely on the host platform for MCP registration and have no automated hook behavior.
- **smelt-agent** is an entirely different domain — it provides instructions for infrastructure agents that dispatch and monitor orchestrated runs rather than performing development work.

The `stop-gate-check.sh` script in claude-code's scripts directory appears to be the original version superseded by `cycle-stop-check.sh`. Only `cycle-stop-check.sh` is referenced in `hooks.json`.
