# Features Research — v0.1.0 Vertical Slice

**Research Date:** 2026-02-28
**Scope:** What features are expected for Assay v0.1.0?
**North Star:** "Agent reads a spec, does work, hits a gate, gets a result."

---

## Executive Summary

Research across spec-driven development tools (spec-kit, OpenSpec, Kiro), quality gate systems (agtx, agent-orchestrator), MCP server ecosystems (rmcp, official spec), and Claude Code plugin format (official docs) reveals a clear feature set for Assay's v0.1.0 vertical slice.

The core loop — config init, TOML spec files, command gate evaluation, MCP server, Claude Code plugin — aligns with industry patterns while Assay's dual-track gate differentiator (deterministic + agent-evaluated) is genuinely novel. No existing tool combines structured gate results with MCP-exposed evaluation.

---

## 1. Config & Project Initialization

### How Similar Tools Work

| Tool | Init Command | Creates | Config Format |
|------|-------------|---------|---------------|
| cargo | `cargo init` | `Cargo.toml`, `src/main.rs`, `.gitignore` | TOML |
| npm | `npm init` | `package.json` | JSON |
| agtx | auto on first use | `~/.config/agtx/config.toml`, `.agtx/config.toml` | TOML |
| spec-kit | `speckit init` | `specs/` directory, constitution file | Markdown |
| OpenSpec | `opsx:init` | `openspec/changes/` directory structure | Markdown |
| turbo | `turbo init` | `turbo.json` | JSON |

### Expected Behavior

**Project init (`assay init`) should:**
- Create a dot-directory (`.assay/`) with config and spec subdirectories
- Generate a minimal `config.toml` with project name and sensible defaults
- Create a `specs/` directory with an example spec file
- Be idempotent — refuse to overwrite existing config, or merge gracefully
- Work in existing projects (not just greenfield)

**Config file (`config.toml`) should contain:**
- Project name (inferred from directory name or prompted)
- Spec directory path (default: `.assay/specs/`)
- Gate defaults (timeout, working directory)
- Schema version for forward compatibility

**Two-level config pattern** (from agtx): global defaults in `~/.config/assay/config.toml`, project overrides in `.assay/config.toml`. v0.1.0 only needs project-level; global can be deferred.

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| `assay init` creates `.assay/` directory | Table Stakes | Low | None | Standard CLI init pattern |
| `config.toml` generation with defaults | Table Stakes | Low | Config types in assay-types | Infer project name from directory |
| Example spec file in `specs/` | Table Stakes | Low | Spec types | Bootstraps first-use experience |
| Idempotent init (skip existing) | Table Stakes | Low | Filesystem checks | Prevent accidental overwrites |
| Config validation on load | Table Stakes | Medium | Validation functions in core | trim-then-validate pattern |
| Global config (`~/.config/assay/`) | Differentiator | Medium | Config merge logic | Defer to v0.2 |
| Interactive init wizard | Anti-Feature | — | — | YAGNI for agent-first tool |

---

## 2. Spec-Driven Development (TOML Spec Files)

### How Existing Tools Define Specs

**spec-kit (GitHub):**
- Markdown files with structured sections (user stories, acceptance criteria)
- Rigid phase gates: spec -> plan -> tasks -> implement
- `/speckit.analyze` performs consistency checks across artifacts
- Acceptance criteria are prose-based, testable statements
- Clarification markers (`[NEEDS CLARIFICATION]`) for ambiguities

**OpenSpec (Fission AI):**
- Markdown-based: `proposal.md`, `specs/`, `design.md`, `tasks.md`
- Criteria emerge through proposal justification and specification scenarios
- Fluid iteration — any artifact can be updated anytime, no phase gates
- Slash command integration with 20+ AI tools

**agtx:**
- Tasks defined through a kanban editor with inline file references
- Phase completion signaled by artifact files (`.agtx/plan.md`, `.agtx/execute.md`)
- Plugin system for different spec methodologies (spec-kit, gsd, void)

### Assay's Approach (TOML)

Assay intentionally uses TOML instead of Markdown for specs. This is a deliberate choice: specs are machine-readable config, not prose documents. The format prioritizes:
- Parseable by both agents and tools (no Markdown parsing ambiguity)
- Serde-friendly (direct deserialization to Rust types)
- JSON Schema generation via schemars (validation, IDE hints)
- Forward-compatible with `prompt` field for agent-evaluated criteria

### Expected Spec Format

```toml
# .assay/specs/example.toml
name = "add-auth-flow"
description = "Implement OAuth2 authentication with JWT tokens"

[[criteria]]
name = "tests-pass"
description = "All test suites pass"
cmd = "cargo test"

[[criteria]]
name = "no-clippy-warnings"
description = "Clippy reports zero warnings"
cmd = "cargo clippy -- -D warnings"

[[criteria]]
name = "auth-endpoint-exists"
description = "A /auth/login endpoint accepts POST with email+password"
# No cmd = agent-evaluated in future (v0.2 prompt field)
```

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| TOML spec file parsing | Table Stakes | Low | `toml` crate, Spec type | Deserialize to assay-types::Spec |
| Criteria with optional `cmd` field | Table Stakes | Low | Spec type design | Forward-compatible with `prompt` |
| Spec validation (name required, unique criteria names) | Table Stakes | Medium | Validation functions | Free functions in assay-core |
| `assay spec show` (display parsed spec) | Table Stakes | Low | Spec parsing | Thin CLI wrapper |
| Spec directory scanning (find all `.toml` in specs/) | Table Stakes | Low | Filesystem + config | Walk `.assay/specs/` |
| Spec metadata (tags, priority, created date) | Differentiator | Low | Type extensions | Nice-to-have but not blocking |
| Markdown spec bodies | Anti-Feature | — | — | Prose specs are what spec-kit/OpenSpec do; Assay is structured-first |
| Spec lifecycle/status tracking | Anti-Feature | — | — | Deferred to v0.2 workflow phase |
| Spec dependencies/versioning | Anti-Feature | — | — | Deferred; complexity too high for v0.1 |

---

## 3. Quality Gate Evaluation

### How Quality Gate Systems Work

**agtx artifact gates:**
- Phase completion detected by presence of artifact files
- Binary signal (file exists = phase done)
- No structured results, no evidence capture

**spec-kit constitutional gates:**
- Multi-phase gates enforcing constitutional principles
- Pre-implementation gates: simplicity, anti-abstraction, integration-first
- Requirement completeness checklist (no clarification markers remain)
- Gates are conceptual — enforced by human/AI review, not automated

**CI/CD quality gates (general pattern):**
- Execute shell commands (test suites, linters, type checks)
- Binary pass/fail based on exit code
- Structured output (JUnit XML, TAP, JSON reports)
- Timeout enforcement
- Parallel execution with dependency ordering

### Assay's Gate Design (Dual-Track, v0.1 = Deterministic Only)

Assay's differentiator is dual-track criteria, but v0.1 ships only the deterministic track:

**Deterministic (v0.1):** Shell commands with exit code evaluation
- Exit code 0 = pass, non-zero = fail
- Capture stdout + stderr as evidence
- Timeout enforcement
- Working directory explicit (not inherited)

**Agent-evaluated (v0.2):** Natural-language assertions verified by AI
- `prompt` field on criteria
- Agent subprocess invocation
- Structured `AgentEvalRequest` / `AgentEvalResponse`
- Feature-flagged (`--features agent-eval`)

### Expected Gate Result Structure

```rust
pub struct GateResult {
    pub spec_name: String,
    pub criterion_name: String,
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub timestamp: DateTime<Utc>,
}
```

Key design: Gate (config) is never mixed with GateResult (runtime state). `passed: bool` does not live on the Gate type.

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| Command gate execution (shell cmd, exit code) | Table Stakes | Medium | `std::process::Command` | Sync in v0.1, async guidance documented |
| Structured GateResult with stdout/stderr evidence | Table Stakes | Medium | GateResult type | Core differentiator data |
| Timeout enforcement on gate commands | Table Stakes | Medium | Process management | Prevent hung gates |
| Explicit working_dir parameter | Table Stakes | Low | GateResult design | Never inherit; always explicit |
| `assay gate run` CLI command | Table Stakes | Low | Gate evaluation in core | Thin wrapper |
| GateKind enum (`Command` variant) | Table Stakes | Low | Type design | `#[serde(tag = "type")]` for forward compat |
| Aggregate gate result (all criteria for a spec) | Differentiator | Medium | Result collection logic | "3/5 criteria passed" summary |
| Parallel gate execution | Differentiator | High | Async runtime | Defer to v0.2; document guidance |
| File existence gate | Differentiator | Low | Filesystem check | Simple but useful; could fit v0.1 |
| Threshold gate (e.g., coverage >= 80%) | Differentiator | Medium | Output parsing | Defer to v0.2 |
| Composite gates (AND/OR logic) | Differentiator | High | Gate composition | Defer to v0.2 |
| Agent-evaluated criteria (`prompt` field) | Differentiator | High | Agent subprocess, types | Defer to v0.2; type design in v0.1 |
| Gate caching/memoization | Anti-Feature | — | — | Gates must re-evaluate; freshness matters |
| Gate ordering/dependencies | Anti-Feature | — | — | YAGNI for v0.1 |

---

## 4. MCP Server

### MCP Protocol Overview

The Model Context Protocol defines three server primitives:
- **Tools**: Executable functions the AI model can invoke (model-controlled)
- **Resources**: Data sources providing context (application-controlled)
- **Prompts**: Reusable interaction templates

For Assay v0.1.0, **tools are the correct primitive**. The agent decides when to read a spec or run a gate — these are model-controlled actions, not static context.

### MCP Transport

- **stdio** (v0.1.0): Standard input/output streams. Optimal for local CLI integration. No network overhead. This is what Claude Code, Codex, and other local agents use.
- **Streamable HTTP** (future): For remote servers. Not needed in v0.1.

### MCP Lifecycle

1. Client sends `initialize` with `protocolVersion` and `capabilities`
2. Server responds with its capabilities (`tools`, optionally `resources`)
3. Client sends `notifications/initialized`
4. Client calls `tools/list` to discover tools
5. Client calls `tools/call` with tool name + arguments
6. Server returns `CallToolResult` with content array

### MCP Tool Design Best Practices

From ecosystem research:

| Practice | Guidance | Source |
|----------|----------|--------|
| Naming | snake_case, `{service}_{action}_{resource}` pattern | 95% of MCP tools use snake_case |
| Argument design | Flat top-level primitives, not nested objects | Prevents agent hallucination of key structures |
| Descriptions | Specify WHEN to use, argument format, response format | Every text string is part of the agent's context |
| Error messages | Helpful strings, not raw exceptions | Allow agents to self-correct |
| Tool count | 5-15 per server, "one server, one job" | Context window constraint |
| Schema | JSON Schema via `inputSchema`, use enums where possible | schemars generates this for free |

### Assay MCP Tool Design

**`spec/get`** — Retrieve a parsed spec by name

```json
{
  "name": "spec_get",
  "description": "Get the full specification including all criteria. Use this to understand what needs to be built and what gates will evaluate.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Spec file name (without .toml extension) or full path"
      }
    },
    "required": ["name"]
  }
}
```

**`gate/run`** — Execute all command gates for a spec

```json
{
  "name": "gate_run",
  "description": "Run all command-based quality gates for a spec. Returns structured pass/fail results with stdout/stderr evidence for each criterion. Use after completing implementation work to verify quality.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "spec_name": {
        "type": "string",
        "description": "Name of the spec to evaluate gates for"
      },
      "working_dir": {
        "type": "string",
        "description": "Working directory for command execution. Defaults to project root."
      }
    },
    "required": ["spec_name"]
  }
}
```

### rmcp Implementation Pattern (Rust)

The official Rust MCP SDK (rmcp) provides:
- `#[tool_router]` macro on impl block — auto-generates tool routing
- `#[tool]` macro on methods — marks functions as callable tools
- `Parameters<T>` — typed input deserialization (T must impl `Deserialize` + `JsonSchema`)
- `CallToolResult::success(vec![Content::text(...)])` — standard response
- `Json<T>` wrapper — structured JSON output
- `ServerHandler` trait — requires `get_info()` returning `ServerInfo` with capabilities
- `#[tool_handler]` macro — generates `tool_call` and `list_tools` implementations
- `.serve(stdio()).await` — starts stdio transport

```rust
#[tool_router]
impl AssayServer {
    #[tool(description = "Get specification by name")]
    async fn spec_get(&self, params: Parameters<SpecGetRequest>)
        -> Result<Json<Spec>, McpError> { /* ... */ }

    #[tool(description = "Run quality gates for a spec")]
    async fn gate_run(&self, params: Parameters<GateRunRequest>)
        -> Result<Json<Vec<GateResult>>, McpError> { /* ... */ }
}
```

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| stdio transport via rmcp | Table Stakes | Medium | rmcp crate, tokio | Standard for local MCP servers |
| `spec_get` tool (retrieve parsed spec) | Table Stakes | Low | Spec parsing in core | Read-only, no side effects |
| `gate_run` tool (execute gates, return results) | Table Stakes | Medium | Gate evaluation in core | Side effects (runs commands) |
| JSON Schema on tool inputs (via schemars) | Table Stakes | Low | schemars derives | Free with Parameters<T> |
| Structured JSON responses (not just text) | Table Stakes | Low | Json<T> wrapper | Agents parse structured data better |
| Helpful error messages (not raw panics) | Table Stakes | Low | Error mapping | Agents need actionable errors |
| `assay mcp serve` CLI subcommand | Table Stakes | Low | MCP server in core | Thin CLI entry point |
| ServerInfo with instructions field | Table Stakes | Low | ServerHandler impl | Tells agent what this server does |
| `spec_list` tool (enumerate available specs) | Differentiator | Low | Spec directory scanning | Useful for discovery; easy to add |
| `gate_status` tool (last gate results) | Differentiator | Medium | Result persistence | Requires some state; defer if complex |
| Resource: project config | Differentiator | Low | Config loading | Could expose config as MCP resource |
| Prompts: gate review template | Differentiator | Low | Prompt template | Help agents structure review |
| SSE/HTTP transport | Anti-Feature | — | — | v0.1 is local-only; stdio sufficient |
| Tool notifications (listChanged) | Anti-Feature | — | — | Static tool set in v0.1 |
| Sampling (ask host LLM) | Anti-Feature | — | — | Not needed; agent already has LLM |

---

## 5. Claude Code Plugin

### Plugin Format (Official Spec, 2026)

A Claude Code plugin is a directory with this structure:

```
assay-plugin/
  .claude-plugin/
    plugin.json            # Manifest (name, version, description, author)
  commands/                # Legacy slash commands (Markdown files)
  skills/                  # Skills with SKILL.md (preferred over commands/)
    gate-check/
      SKILL.md
  agents/                  # Subagent definitions (Markdown files)
  hooks/
    hooks.json             # Event handlers (PreToolUse, PostToolUse, Stop, etc.)
  .mcp.json                # MCP server configuration
  .lsp.json                # LSP server configuration (optional)
  settings.json            # Default plugin settings
  scripts/                 # Hook and utility scripts
```

### Key Components

**plugin.json** (only required field is `name`):
```json
{
  "name": "assay",
  "version": "0.1.0",
  "description": "Spec-driven quality gates for agentic development",
  "author": { "name": "wollax" },
  "repository": "https://github.com/wollax/assay",
  "license": "MIT",
  "keywords": ["quality-gates", "spec-driven", "mcp"]
}
```

**.mcp.json** (connects Claude Code to the Assay MCP server):
```json
{
  "mcpServers": {
    "assay": {
      "command": "${CLAUDE_PLUGIN_ROOT}/../../target/release/assay-cli",
      "args": ["mcp", "serve"],
      "cwd": "${CLAUDE_PLUGIN_ROOT}"
    }
  }
}
```

**Hooks** (available events):
- `PreToolUse` / `PostToolUse` / `PostToolUseFailure` — before/after tool execution
- `Stop` / `SubagentStop` — when Claude attempts to stop
- `UserPromptSubmit` — when user submits a prompt
- `SessionStart` / `SessionEnd` — session lifecycle
- `PreCompact` — before conversation history compaction
- `TaskCompleted` — when a task is marked complete
- `TeammateIdle` — when a team teammate goes idle

Hook types: `command` (shell script), `prompt` (LLM evaluation), `agent` (agentic verification).

**Skills** (preferred over commands/):
```markdown
# skills/gate-check/SKILL.md
---
name: gate-check
description: Run quality gates for the current spec and report results
---

When invoked, use the assay MCP server to:
1. Call `spec_get` to load the current spec
2. Call `gate_run` to execute all quality gates
3. Report results with pass/fail for each criterion
4. If any gate fails, suggest specific fixes based on stdout/stderr evidence
```

**Path variable**: `${CLAUDE_PLUGIN_ROOT}` resolves to the plugin's installed location. All intra-plugin paths must use this.

**Installation scopes**: `user` (global), `project` (shared via VCS), `local` (gitignored).

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| `plugin.json` manifest | Table Stakes | Low | None (static JSON) | Name, version, description, author |
| `.mcp.json` pointing to assay MCP server | Table Stakes | Low | MCP server binary | `${CLAUDE_PLUGIN_ROOT}` for paths |
| `/gate-check` skill (run gates, report results) | Table Stakes | Low | MCP server working | SKILL.md with instructions |
| `/spec-show` skill (display current spec) | Table Stakes | Low | MCP server working | Simple read-only skill |
| CLAUDE.md snippet for project integration | Differentiator | Low | None | Tells Claude how to use Assay in-project |
| PostToolUse hook (auto-gate after Write/Edit) | Differentiator | Medium | Hook scripts, gate evaluation | Auto-quality-check after code changes |
| Stop hook (gate before completion) | Differentiator | Medium | Hook scripts | Prevent agent from stopping without passing gates |
| Agent definition (quality reviewer) | Differentiator | Low | Agent Markdown file | Specialized subagent for reviews |
| settings.json defaults | Differentiator | Low | Plugin settings | Pre-configure plugin behavior |
| LSP integration | Anti-Feature | — | — | No language server needed for gate results |
| Complex hook chains | Anti-Feature | — | — | Keep hooks simple in v0.1 |
| Plugin marketplace publishing | Anti-Feature | — | — | Premature; need working product first |

---

## 6. Schema Generation

### Expected Behavior

JSON Schema generation via schemars is already set up (derives on types). The pipeline needs:
- A standalone binary or script that generates schemas for all public types
- Output to `schemas/` directory as `.json` files
- A `just schemas` command to regenerate
- Schemas used for: MCP tool input validation, IDE hints, documentation, external tooling

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| Schema generation binary | Table Stakes | Low | schemars, existing types | Example binary or build script |
| `just schemas` command | Table Stakes | Low | Justfile entry | Regenerate on demand |
| Schemas for Spec, Config, GateResult | Table Stakes | Low | Type derives | Already have schemars derives |
| Schema for MCP tool inputs | Differentiator | Low | MCP request types | Validates agent-provided inputs |
| Schema versioning | Anti-Feature | — | — | YAGNI for v0.1 |

---

## 7. Error Handling

### Expected Behavior

Unified error type using thiserror with `#[non_exhaustive]` for forward compatibility.

### Features

| Feature | Category | Complexity | Dependencies | Notes |
|---------|----------|------------|-------------|-------|
| `AssayError` enum with thiserror | Table Stakes | Low | thiserror (already in workspace) | `#[non_exhaustive]` from day 1 |
| `Io` variant (filesystem, process) | Table Stakes | Low | std::io::Error | First consumer |
| `Config` variant (parse/validation) | Table Stakes | Low | Config loading | Second consumer |
| `Spec` variant (parse/validation) | Table Stakes | Low | Spec loading | Third consumer |
| `Gate` variant (execution failures) | Table Stakes | Medium | Gate evaluation | Timeout, command not found, etc. |
| `Mcp` variant (protocol errors) | Table Stakes | Low | MCP server | Map rmcp errors |
| Result type alias | Table Stakes | Low | AssayError | `pub type Result<T> = std::result::Result<T, AssayError>` |
| Error context/source chaining | Differentiator | Low | thiserror `#[from]` | Good diagnostics |
| Error codes for MCP responses | Differentiator | Low | Error mapping | Agents need structured error info |

---

## Cross-Cutting Themes

### 1. Gates Are The Product

Every research source confirmed: quality gates are Assay's category-defining feature. The MCP server exists to expose gates. The plugin exists to trigger gates. Config and specs exist to feed gates. Everything serves the gate evaluation loop.

### 2. Agent-First, Human-Second

MCP tool design best practices emphasize: design for what the agent needs to achieve, not what a human would click. This means:
- Structured JSON responses over pretty-printed text
- Flat argument schemas over nested objects
- Helpful error strings over stack traces
- Tool descriptions that say WHEN to use, not just WHAT it does

### 3. Start Minimal, Grow via Consumers

The rmcp pattern of `Parameters<T>` + `Json<T>` + schemars means types drive everything. Add fields to types, and MCP schemas update automatically. This aligns with Assay's type-first architecture.

### 4. Plugin Is a Research Instrument

The brainstorm identified the plugin as a parallel research track for learning agent UX patterns. v0.1.0 plugin should be functional enough to discover: what skills do agents actually invoke? What hook events matter? What information do agents need from gate results?

---

## Dependency Map

```
Error Types ─────────────────────────────────────────────┐
     │                                                    │
     ▼                                                    │
Domain Model Hardening (GateKind, GateResult)             │
     │                                                    │
     ├──────────────┬──────────────┐                      │
     ▼              ▼              ▼                      │
Config Loading   Spec Parsing   Schema Gen                │
     │              │                                     │
     ├──────────────┤                                     │
     ▼              ▼                                     │
Spec + Config Validation                                  │
     │                                                    │
     ▼                                                    │
Gate Evaluation (command gates)                           │
     │                                                    │
     ├──────────────────────────────┐                     │
     ▼                              ▼                     │
CLI Subcommands              MCP Server (rmcp)            │
  (init, validate,             (spec_get, gate_run)       │
   gate run, spec show,        │                          │
   mcp serve)                  ▼                          │
                         Claude Code Plugin               │
                           (.mcp.json, skills,            │
                            hooks)◄───────────────────────┘
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| rmcp API instability | Medium | High | Days 1-2 spike; GO/NO-GO decision |
| Claude Code plugin format changes | Low | Medium | Plugin format is well-documented and stable |
| Gate timeout edge cases | Medium | Low | Conservative defaults, well-tested |
| TOML spec format too rigid | Low | Medium | Optional fields, `#[serde(default)]` |
| MCP tool descriptions insufficient for agents | Medium | Medium | Iterate based on plugin research track |

---

## Summary: v0.1.0 Feature Set

### Table Stakes (Must Ship)

1. **Error types** — `AssayError` with thiserror, `#[non_exhaustive]`
2. **Domain model** — `GateKind` enum, `GateResult` with evidence fields
3. **Config init** — `assay init` creates `.assay/config.toml` + `specs/`
4. **Config loading** — TOML parse, validation, free functions in core
5. **Spec files** — TOML with criteria, optional `cmd` field
6. **Spec validation** — Name required, unique criteria, trim-then-validate
7. **Gate evaluation** — Command execution, exit code, stdout/stderr capture, timeout
8. **CLI commands** — `init`, `validate`, `gate run`, `spec show`, `mcp serve`
9. **MCP server** — stdio via rmcp, `spec_get` + `gate_run` tools
10. **Claude Code plugin** — `plugin.json` + `.mcp.json` + gate-check skill
11. **Schema generation** — Standalone binary + `just schemas`

### Differentiators (Should Ship If Time Allows)

1. **`spec_list` MCP tool** — Enumerate available specs (easy, high value)
2. **Aggregate gate results** — "3/5 passed" summary per spec
3. **PostToolUse hook** — Auto-gate after code changes
4. **Stop hook** — Prevent agent completion without passing gates
5. **CLAUDE.md snippet** — In-project instructions for Assay-aware agents
6. **Error codes in MCP responses** — Structured error info for agents

### Anti-Features (Explicitly Out of Scope)

1. **Interactive init wizard** — Agent-first tool; non-interactive by default
2. **Markdown spec bodies** — Assay is structured-first, not prose-first
3. **Spec lifecycle/status** — Deferred to v0.2 workflow phase
4. **SSE/HTTP transport** — v0.1 is local-only
5. **Gate caching** — Gates must re-evaluate fresh each time
6. **Plugin marketplace** — Need working product before distribution
7. **Agent-evaluated criteria** — v0.2 dual-track; type design only in v0.1
8. **Composite gates (AND/OR)** — v0.2 gate composition
9. **Parallel gate execution** — Document async guidance, ship sync in v0.1

---

## Sources

- [Claude Code Plugins Reference](https://code.claude.com/docs/en/plugins-reference)
- [MCP Architecture Overview](https://modelcontextprotocol.io/docs/learn/architecture)
- [MCP Server Best Practices](https://www.philschmid.de/mcp-best-practices)
- [MCP Tool Naming Conventions](https://zazencodes.com/blog/mcp-server-naming-conventions)
- [rmcp (Rust MCP SDK)](https://docs.rs/rmcp)
- [GitHub spec-kit](https://github.com/github/spec-kit/blob/main/spec-driven.md)
- [OpenSpec](https://github.com/Fission-AI/OpenSpec)
- [agtx](https://github.com/fynnfluegge/agtx)
- [Spec-Driven Development (Thoughtworks)](https://thoughtworks.medium.com/spec-driven-development-d85995a81387)
- [Spec-Driven Development Tools Comparison (Martin Fowler)](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html)
- [MCP Features Guide (WorkOS)](https://workos.com/blog/mcp-features-guide)

---

*Research completed: 2026-02-28*
