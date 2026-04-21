# Architecture: Plugins

The Assay plugin system provides agent-facing instructions, skills, and platform integrations for four target environments. Plugins contain no business logic. They instruct AI coding agents on how to call MCP tools in the correct sequence.

---

## Overview

Four plugins ship in the `plugins/` directory:

| Plugin | Target Platform | Domain | Maturity |
|--------|----------------|--------|----------|
| **claude-code** | Claude Code (Anthropic) | Developer workflow | Reference implementation. Full integration: MCP config, hooks, skills, checkpoint automation |
| **codex** | OpenAI Codex | Developer workflow | Skill-only. No hooks, no manifest, no MCP config |
| **opencode** | OpenCode | Developer workflow | Skill-only with NPM package scaffolding. No hooks, no MCP config |
| **smelt-agent** | Smelt worker agents | Infrastructure | Documentation-only. Orchestrated run dispatch, peer discovery, signal forwarding |

The three developer-facing plugins (claude-code, codex, opencode) share the same six active skills and core workflow. They differ in platform integration depth. smelt-agent is an entirely separate domain focused on infrastructure agents that dispatch and monitor orchestrated runs.

---

## Plugin Comparison Matrix

| Aspect | claude-code | codex | opencode | smelt-agent |
|--------|-------------|-------|----------|-------------|
| **MCP registration** | `.mcp.json` (explicit) | External (host config) | External (host config) | External (host config) |
| **Hook system** | 4 hooks (PostToolUse, PreCompact, Stop) | None | None | None |
| **Gate enforcement** | Automated via Stop hook | Manual (agent follows workflow) | Manual (agent follows workflow) | N/A (worker agent) |
| **Checkpoint persistence** | Automated via hooks | None | None | N/A |
| **Active skills** | 6 | 6 | 6 | 4 |
| **Deprecated skills** | 3 | 3 | 3 | 0 |
| **Plugin manifest** | `plugin.json` | None | `opencode.json` + `package.json` | None |
| **Agent instructions** | `CLAUDE.md` | `AGENTS.md` | `AGENTS.md` | `AGENTS.md` |
| **Skill format** | `skills/<name>/SKILL.md` | `skills/<name>.md` | `skills/<name>.md` | `skills/<name>.md` |
| **Installation** | `claude plugin add` | Symlink skills dir | Register in OpenCode config | Agent instructions only |
| **TypeScript scaffolding** | No | No | Yes (no source yet) | No |
| **Test infrastructure** | No | No | No | Yes (`verify-docs.sh`) |
| **Config files** | 3 | 0 | 3 | 0 |
| **Shell scripts** | 4 | 0 | 0 | 0 |

---

## MCP Integration

### Registration

Only claude-code has an explicit MCP configuration file (`.mcp.json`):

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

This launches the `assay` binary as a stdio MCP server. The binary must be on PATH.

Codex and opencode document the MCP tool table in their `AGENTS.md` but rely on the host platform to register the MCP server externally. The plugin's role for these platforms is limited to skill definitions and agent instructions.

### Shared Tool Table (Developer Plugins)

All three developer-facing plugins reference these MCP tools:

| Tool | Description | claude-code | codex | opencode |
|------|-------------|-------------|-------|----------|
| `spec_list` | List all specs | Yes | Yes | Yes |
| `spec_get` | Get a spec's full definition | Yes | Yes | Yes |
| `spec_create` | Create a spec with criteria | Yes | Yes | Yes |
| `gate_run` | Run quality gates | Yes | Yes | Yes |
| `cycle_status` | Get active milestone progress | Yes | Yes | Yes |
| `cycle_advance` | Advance the active chunk | Yes | Yes | Yes |
| `chunk_status` | Get gate results for a chunk | Yes | Yes | Yes |
| `milestone_list` | List all milestones | Yes | Yes | Yes |
| `milestone_get` | Get a milestone by slug | Yes | -- | -- |
| `milestone_create` | Create a new milestone | Yes | Yes | Yes |
| `pr_create` | Create a gate-gated PR | Yes | Yes | Yes |

### Smelt-Agent Additional Tools

| Tool | Description |
|------|-------------|
| `run_manifest` | Execute a single-session manifest |
| `orchestrate_run` | Launch a multi-session orchestrated run |
| `orchestrate_status` | Query orchestrated run status |
| `poll_signals` | Read PeerUpdate messages from signal inbox |
| `send_signal` | POST a SignalRequest to any signal endpoint |
| `merge_propose` | Push branch and create PR with gate evidence |

---

## Skill System

### Developer Skills (claude-code, codex, opencode)

Six active skills with three deprecated aliases. Skill content is logically identical across the three plugins (codex/opencode `plan` adds slug collision checking and manual `cmd` warning).

| Skill | Invocation | Purpose |
|-------|-----------|---------|
| **explore** | `/assay:explore` | Load lean project context (~500 tokens). Tiered loading: always loads cycle_status + spec_list; conditionally loads chunk_status + git log. |
| **plan** | `/assay:plan` | Interview-guided milestone creation. Supports `quick` mode (claude-code) for flat single-chunk specs. Collects all inputs before calling MCP tools. |
| **focus** | `/assay:focus` | Show active work context: milestone, chunk, criteria, gate status, progress. Hides milestone/chunk terminology for quick milestones. |
| **check** | `/assay:check [name]` | Run quality gates with smart routing. Auto-detects active chunk if no name given. Reports pass/fail with next-step suggestions. |
| **ship** | `/assay:ship` | Verify gates pass, then create PR with gate evidence via `pr_create`. Blocks PR creation on gate failures. |
| **spec-show** | `/assay:spec-show [name]` | Display a spec's full definition. Groups criteria by type (executable vs descriptive). |

| Deprecated Skill | Redirects To |
|-----------------|--------------|
| `gate-check` | `check` |
| `next-chunk` | `focus` |
| `status` / `cycle-status` | `focus` |

### Smelt-Agent Skills (4 total)

| Skill | Invocation | Purpose |
|-------|-----------|---------|
| **run-dispatch** | `/assay:run-dispatch` | Read RunManifest, configure StateBackendConfig, choose single or multi-session dispatch |
| **backend-status** | `/assay:backend-status` | Query and interpret OrchestratorStatus (phases, per-session states, capability degradation) |
| **peer-message** | `/assay:peer-message` | Inter-session communication: mesh direct messaging, gossip knowledge manifest, HTTP signals |
| **peer-registry** | `/assay:peer-registry` | Multi-machine peer discovery, registration lifecycle, cross-instance signal forwarding |

### Skill Format

| Platform | Path Pattern | Frontmatter |
|----------|-------------|-------------|
| claude-code | `skills/<name>/SKILL.md` | YAML (name, description) |
| codex | `skills/<name>.md` | Mixed (some have YAML, some plain) |
| opencode | `skills/<name>.md` | Mixed (same as codex) |
| smelt-agent | `skills/<name>.md` | YAML (name, description) |

---

## Hook System

Only claude-code defines hooks. The other three plugins have no hook behavior.

### Hook Events

| Event | Matcher | Script | Timeout | Purpose |
|-------|---------|--------|---------|---------|
| PostToolUse | `Write\|Edit` | `post-tool-use.sh` | 5s | Nudge agent to run gates after file edits |
| PostToolUse | `Task\|TaskCreate\|TaskUpdate` | `checkpoint-hook.sh` | 5s | Save team state on task operations |
| PreCompact | (all) | `checkpoint-hook.sh` | 10s | Save checkpoint before context compaction |
| Stop | (all) | `cycle-stop-check.sh` | 120s | Block stop if quality gates fail |
| Stop | (all) | `checkpoint-hook.sh` | 10s | Save checkpoint on stop |

### Guard Pattern

All hook scripts follow a consistent 5-guard pattern that fails open (order matters):

1. `jq` not installed -- allow (can't parse JSON)
2. `stop_hook_active = true` -- allow (prevents infinite loops)
3. `ASSAY_STOP_HOOK_MODE=off` -- allow (user disabled)
4. No `.assay/` directory -- allow (not an Assay project)
5. `assay` binary not on PATH -- allow (not installed)

### Script Behaviors

**`post-tool-use.sh`** -- Reminder-only. Does not run gates. Outputs a `hookSpecificOutput` JSON message nudging the agent to run `/assay:check`. Detects active milestone/chunk via `assay milestone status --json`.

**`checkpoint-hook.sh`** -- Saves team state via `assay checkpoint save`. 5-second debounce. Fire-and-forget (spawns in background, always exits 0). Never blocks the agent.

**`cycle-stop-check.sh`** -- Primary gate enforcement. Discovers incomplete chunks, runs per-chunk gate checks. Three modes via `ASSAY_STOP_HOOK_MODE`:

| Mode | Behavior |
|------|----------|
| `enforce` (default) | Blocks stop with `decision: "block"` JSON naming failing chunks |
| `warn` | Allows stop but injects `systemMessage` warning |
| `off` | Disables entirely |

Falls back to `assay gate run --all` when no active milestone exists.

---

## Agent Instructions

Each plugin provides an instructions file that frames the agent's role and available tools.

### claude-code (`CLAUDE.md`)

Structured as a reference card:
1. Skills table -- 6 active skills with invocation syntax
2. CLI commands table -- 5 milestone management and PR commands
3. MCP tools table -- 11 tools with descriptions

### codex / opencode (`AGENTS.md`)

Identical content across both plugins:
1. Preamble explaining the spec/criteria/gate model
2. Skills table -- 6 active skills
3. MCP tools table -- 10 tools (omits `milestone_get`)
4. Workflow -- 6-step numbered process: Plan, Read, Implement, Gate-check, Advance, PR

### smelt-agent (`AGENTS.md`)

Distinct from the developer plugins:
1. Role framing -- "You are a smelt worker agent executing Assay runs"
2. Skills table -- 4 infrastructure skills
3. MCP tools table -- 11 tools including smelt-specific ones
4. Workflow -- 6-step run execution: Receive manifest, Configure backend, Dispatch, Monitor, Handle messaging, Report
5. Backend capability awareness -- CapabilitySet flags documentation
6. Cross-instance signal forwarding -- Protocol and `X-Assay-Forwarded` loop prevention
7. Environment variables -- `ASSAY_SIGNAL_PORT`, `ASSAY_SIGNAL_BIND`, `ASSAY_SIGNAL_URL`, `ASSAY_SIGNAL_TOKEN`

---

## Cross-Plugin Patterns

### Shared Conventions

1. **Skill naming**: All plugins use `/assay:<skill-name>` invocation. Names are consistent across platforms.

2. **Deprecation strategy**: All developer plugins maintain deprecated aliases (`gate-check` to `check`, `next-chunk` to `focus`, `status`/`cycle-status` to `focus`) with deprecation notices. Old names resolve to new implementations.

3. **MCP-only business logic**: No plugin embeds domain logic. They provide instructions for agents to call MCP tools in the correct sequence. All state mutation flows through the Assay binary.

4. **Graceful degradation**: Hook scripts (claude-code) and capability checks (smelt-agent) fail open. Missing binaries, directories, or unsupported features allow the workflow to continue.

5. **Content parity**: The six active developer skills have identical logical content across claude-code, codex, and opencode (with minor platform-specific variations in the `plan` skill).

### Architectural Layering

```
claude-code  ──┐
codex        ──┼──> Shared skill definitions ──> MCP tools ──> assay binary
opencode     ──┘

smelt-agent  ──────> Infrastructure skills ──> MCP tools ──> assay binary
```

Plugins are a presentation layer. They translate platform-specific conventions (CLAUDE.md vs AGENTS.md, skill subdirectories vs flat files, hook systems vs manual workflow) into a common interface: instructing agents to call the same MCP tools.

---

## Configuration

### claude-code

| Setting | Source | Default | Description |
|---------|--------|---------|-------------|
| `ASSAY_STOP_HOOK_MODE` | Env var | `enforce` | Stop hook behavior: `enforce`, `warn`, `off` |
| Plugin path | `claude plugin add <path>` | -- | Plugin installation |
| `assay` binary | PATH | Required | Must be available as `assay` command |
| `jq` binary | PATH | Optional | Required for JSON in hooks; degrades gracefully |

### codex

No configuration. Skills are installed via symlink:
```
ln -s /path/to/plugins/codex/skills .agents/skills/assay
```

### opencode

| Setting | Source | Description |
|---------|--------|-------------|
| Plugin registration | `opencode.json` | Registered as `@assay/opencode-plugin` |
| NPM package | `package.json` | `"type": "module"`, `"private": true` |

TypeScript scaffolding (`tsconfig.json` targeting ES2022) exists but has no source files yet.

### smelt-agent

| Setting | Source | Default | Description |
|---------|--------|---------|-------------|
| `ASSAY_SIGNAL_PORT` | Env var | `7432` | HTTP signal listener port |
| `ASSAY_SIGNAL_BIND` | Env var | `127.0.0.1` | Bind address (`0.0.0.0` for multi-machine) |
| `ASSAY_SIGNAL_URL` | Env var | Derived | Override peer-registered URL |
| `ASSAY_SIGNAL_TOKEN` | Env var | None | Optional bearer token |
| `LINEAR_API_KEY` | Env var | None | Required for Linear backend |
| State backend | RunManifest `[state_backend]` | `local_fs` | Orchestrator state persistence |

### State Backend Types (smelt-agent)

| Type | Config Fields | Status |
|------|---------------|--------|
| `local_fs` | (none) | Default, fully implemented |
| `linear` | `team_id`, optional `project_id` | Stub (logs warning, no-op) |
| `github` | `repo`, optional `label` | Stub (logs warning, no-op) |
| `ssh` | `host`, `remote_assay_dir`, optional `user`/`port` | Stub (logs warning, no-op) |
| `smelt` | `url`, `job_id`, optional `token` | Fully implemented |
| `custom` | `name`, `config` | Falls back to no-op |
