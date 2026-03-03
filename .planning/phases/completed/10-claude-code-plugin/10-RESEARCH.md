# Phase 10: Claude Code Plugin - Research

**Researched:** 2026-03-02
**Domain:** Claude Code plugin system (skills, hooks, MCP integration)
**Confidence:** HIGH

## Summary

This phase delivers the Claude Code plugin that ties together the MCP server (Phase 8) and CLI (Phase 9) into a cohesive agent experience. The plugin system is well-documented by Anthropic and the Assay codebase already has the scaffolding (`plugins/claude-code/` with `plugin.json`, empty `hooks/`, `skills/`, `agents/`, `commands/` directories).

The plugin needs five deliverables: (1) `.mcp.json` pointing to `assay mcp serve`, (2) `/gate-check` skill that invokes MCP `gate_run`, (3) `/spec-show` skill that invokes MCP `spec_get`, (4) a CLAUDE.md workflow snippet, (5) `hooks.json` with a PostToolUse reminder hook and a Stop enforcement hook. All of these are static files -- no Rust code changes required for the plugin itself.

The CONTEXT.md decisions constrain the design significantly: PostToolUse is reminder-only (not auto-execution), Stop hook defaults to hard block but is configurable, CLAUDE.md is prescriptive with spec-first workflow, binary path is configurable with PATH lookup default.

**Primary recommendation:** Implement all plugin files as static content. The skills instruct Claude to call MCP tools (`gate_run`, `spec_get`). Hooks are shell scripts using `jq` to parse JSON input and `assay` CLI for gate evaluation. No new Rust code needed.

## Standard Stack

### Core

| Component | Format | Purpose | Why Standard |
| --- | --- | --- | --- |
| `.claude-plugin/plugin.json` | JSON | Plugin manifest (already exists) | Required by Claude Code plugin system |
| `.mcp.json` | JSON | MCP server registration | Standard Claude Code MCP config format |
| `skills/*/SKILL.md` | YAML frontmatter + Markdown | Skill definitions | Claude Code Agent Skills standard |
| `hooks/hooks.json` | JSON | Hook configuration | Claude Code hooks format |
| `CLAUDE.md` snippet | Markdown | Project-level agent instructions | Claude Code memory/instruction system |

### Supporting

| Tool | Purpose | When Used |
| --- | --- | --- |
| `jq` | Parse JSON stdin in hook scripts | PostToolUse and Stop hooks read JSON input |
| `assay` CLI | Run gate checks from hooks | Stop hook runs `assay gate run` to verify gates |
| `${CLAUDE_PLUGIN_ROOT}` | Resolve plugin paths | All hook scripts reference plugin root |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
| --- | --- | --- |
| Shell script hooks | Prompt-based hooks (`type: "prompt"`) | Prompt hooks use LLM evaluation but add latency and token cost; shell scripts are deterministic and free |
| Shell script Stop hook | Agent-based hooks (`type: "agent"`) | Agent hooks can use tools like Read/Grep but are overkill for running a CLI command |
| Skills calling MCP tools | Skills calling `assay` CLI via `allowed-tools: Bash(assay *)` | MCP tools are the intended integration path; CLI invocation bypasses the structured response format |

## Architecture Patterns

### Plugin Directory Structure (Final)

```
plugins/claude-code/
├── .claude-plugin/
│   └── plugin.json              # PLG-01 (already done)
├── .mcp.json                    # PLG-02: MCP server config
├── agents/
│   └── .gitkeep
├── commands/
│   └── .gitkeep
├── hooks/
│   └── hooks.json               # PLG-06 + PLG-07: PostToolUse + Stop hooks
├── scripts/
│   ├── post-tool-use.sh         # PostToolUse reminder script
│   └── stop-gate-check.sh       # Stop hook gate enforcement script
├── skills/
│   ├── gate-check/
│   │   └── SKILL.md             # PLG-03: /gate-check skill
│   └── spec-show/
│       └── SKILL.md             # PLG-04: /spec-show skill
├── CLAUDE.md                    # PLG-05: workflow snippet (template)
└── README.md                    # Updated installation docs
```

### Pattern 1: Skills as MCP Tool Orchestrators

**What:** Skills contain instructions that tell Claude to use MCP tools, not shell commands.
**When to use:** When the plugin provides an MCP server (as Assay does).
**Why:** Skills invoke MCP tools through Claude's natural tool-use mechanism. The MCP server (`gate_run`, `spec_get`, `spec_list`) provides structured JSON responses. Skills instruct Claude how to interpret and present those results.

**Example `/gate-check` SKILL.md:**
```yaml
---
name: gate-check
description: Run quality gates for a spec and report results. Use when checking if code changes meet spec criteria, after implementing features, or when asked about gate status.
---

# Gate Check

Run quality gates for a spec and report structured pass/fail results.

## Instructions

1. If a spec name was provided as an argument, use it. Otherwise, call the `spec_list` MCP tool to discover available specs.
2. Call the `gate_run` MCP tool with the spec name. Set `include_evidence` to `false` for the initial check.
3. Report the results:
   - If ALL criteria passed: report a concise summary (e.g., "3/3 criteria passed for auth-flow")
   - If ANY criteria failed: report each failed criterion with its reason, then call `gate_run` again with `include_evidence: true` to get full stdout/stderr for the failures
4. If multiple specs exist and none was specified, run gates for ALL specs and report aggregate results.
```

Source: Claude Code official docs -- skills invoke MCP tools through Claude's tool-use mechanism, not through direct CLI invocation.

### Pattern 2: Reminder-Only PostToolUse Hook

**What:** PostToolUse hook provides `additionalContext` to Claude after Write/Edit, reminding it to check gates when ready.
**When to use:** Per CONTEXT.md decision -- the hook is reminder-only, not auto-execution.
**Why:** Auto-running gates after every file edit would be slow and wasteful. A reminder lets Claude batch edits and check gates when appropriate.

**Example hooks.json PostToolUse entry:**
```json
{
  "matcher": "Write|Edit",
  "hooks": [
    {
      "type": "command",
      "command": "${CLAUDE_PLUGIN_ROOT}/scripts/post-tool-use.sh",
      "timeout": 5
    }
  ]
}
```

The script outputs JSON with `additionalContext`:
```json
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "File modified. Remember to run /assay:gate-check when ready to verify quality gates."
  }
}
```

Source: Official hooks reference -- PostToolUse hooks return `additionalContext` to inject context for Claude.

### Pattern 3: Stop Hook with Gate Enforcement

**What:** Stop hook runs `assay gate run` and blocks completion if gates fail.
**When to use:** Per CONTEXT.md decision -- defaults to hard block, configurable to warn-and-allow.
**Why:** Prevents the agent from declaring work complete when quality gates are still failing.

**Key details from official docs:**
- Stop hooks receive `stop_hook_active` field (boolean) indicating if Claude is already continuing from a previous Stop hook block. **This is critical for preventing infinite loops.**
- Stop hooks do NOT support matchers -- they fire on every Stop event.
- To block: output JSON with `{"decision": "block", "reason": "..."}` on stdout with exit 0.
- To allow: exit 0 with no output, or exit 0 with empty JSON.
- Exit code 2 also prevents Claude from stopping (stderr fed back as error message).

**Infinite loop prevention:** The `stop_hook_active` field is `true` when Claude is already continuing from a previous Stop hook. The script MUST check this and allow stop if already active (to prevent infinite retry loops).

Source: Official hooks reference -- `stop_hook_active` field documented in Stop input schema.

### Pattern 4: .mcp.json with PATH-based Binary Discovery

**What:** `.mcp.json` uses `"command": "assay"` (PATH lookup) rather than an absolute path.
**When to use:** Per CONTEXT.md decision -- binary path uses PATH lookup by default.

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

Source: Official plugins reference -- `.mcp.json` at plugin root, standard MCP server configuration format.

### Anti-Patterns to Avoid

- **Auto-executing gates on every file edit:** CONTEXT.md explicitly says PostToolUse is reminder-only. Do NOT run `assay gate run` in the PostToolUse hook.
- **Skills that shell out via Bash:** Use MCP tools, not `allowed-tools: Bash(assay *)`. The MCP server provides structured JSON responses that are more token-efficient than parsing CLI output.
- **Stop hook without loop guard:** Always check `stop_hook_active` to prevent infinite continuation loops. If stop_hook_active is true, allow the stop.
- **Blocking stop on missing .assay/ directory:** If the project has no `.assay/` directory, the Stop hook should allow stop (graceful degradation per CONTEXT.md).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
| --- | --- | --- | --- |
| JSON parsing in hooks | Custom parsing with `grep`/`sed` | `jq` | Reliable JSON field extraction, handles edge cases |
| MCP tool invocation from skills | CLI wrapping with Bash | Claude's native MCP tool calls | Skills instruct Claude to use tools; Claude handles MCP protocol |
| Gate result formatting | Custom format logic in skills | MCP `gate_run` response structure | The MCP server already formats bounded, structured responses |
| Plugin installation | Manual file copying | `claude plugin install` / `--plugin-dir` | Standard Claude Code plugin installation mechanism |

**Key insight:** The plugin is almost entirely static content (JSON configs, markdown skills, shell scripts). The heavy lifting is already done by the MCP server (Phase 8) and CLI (Phase 9). This phase wires them together.

## Common Pitfalls

### Pitfall 1: Stop Hook Infinite Loop

**What goes wrong:** Stop hook blocks completion, Claude tries again, Stop hook blocks again, forever.
**Why it happens:** The Stop hook always fires when Claude attempts to stop. If gates are genuinely unfixable (or the agent is stuck), the hook keeps blocking.
**How to avoid:**
1. Check `stop_hook_active` -- if `true`, allow stop (Claude is already in a retry from a previous block).
2. Consider a retry counter (write to a temp file) to allow stop after N consecutive blocks.
**Warning signs:** Claude keeps cycling through "attempting to stop" -> "blocked" -> "attempting to stop" indefinitely.

### Pitfall 2: Missing Binary Error

**What goes wrong:** User installs the plugin but doesn't have `assay` in PATH. MCP server fails to start silently.
**Why it happens:** Plugin uses `"command": "assay"` which requires PATH resolution.
**How to avoid:** The CONTEXT.md decision requires a clear error message with install instructions. The MCP server startup failure should produce a visible error. Additionally, hook scripts should check for the binary before attempting to use it and provide actionable error messages.
**Warning signs:** MCP tools don't appear in Claude's toolkit after plugin installation.

### Pitfall 3: Hook Scripts Not Executable

**What goes wrong:** Hooks silently fail because scripts lack execute permission.
**Why it happens:** Git doesn't always preserve execute bits, especially on Windows or when cloning.
**How to avoid:** Ensure `chmod +x` on all scripts. Include a note in README. Consider using `bash ${CLAUDE_PLUGIN_ROOT}/scripts/script.sh` instead of direct execution (bash prefix doesn't require execute permission).
**Warning signs:** Hooks registered in `/hooks` UI but never fire.

### Pitfall 4: PostToolUse Reminder Fatigue

**What goes wrong:** Claude gets reminded after EVERY Write/Edit and starts running gates too eagerly (defeating the "when ready" intent).
**Why it happens:** The reminder fires after every file change, and Claude may interpret "remember to check gates" as "check gates now."
**How to avoid:** Make the reminder phrasing clearly optional: "When you're done making changes, run /assay:gate-check to verify quality gates." Avoid imperative language like "you must check gates now."
**Warning signs:** Claude calls gate_run after every single file edit instead of batching.

### Pitfall 5: No .assay/ Directory Handling

**What goes wrong:** Stop hook blocks completion in projects without Assay initialized.
**Why it happens:** Hook runs `assay gate run` which fails because there's no `.assay/` directory.
**How to avoid:** Check for `.assay/` directory existence before running gates. If absent, allow stop without blocking. Per CONTEXT.md: "basic commands still work without `.assay/` directory" -- graceful degradation.
**Warning signs:** Stop hook blocks agent in any project where the plugin is installed but Assay isn't initialized.

## Code Examples

### .mcp.json (PLG-02)

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

Source: Claude Code plugins reference -- `.mcp.json` at plugin root, standard format. The `type: "stdio"` field is included per the project's existing `.mcp.json` at repo root.

### hooks.json (PLG-06 + PLG-07)

```json
{
  "description": "Assay quality gate hooks for spec-driven development",
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "bash ${CLAUDE_PLUGIN_ROOT}/scripts/post-tool-use.sh",
            "timeout": 5,
            "statusMessage": "Assay: checking file change"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bash ${CLAUDE_PLUGIN_ROOT}/scripts/stop-gate-check.sh",
            "timeout": 120,
            "statusMessage": "Assay: verifying quality gates"
          }
        ]
      }
    ]
  }
}
```

Source: Claude Code hooks reference -- `description` is an optional top-level field for plugin hooks. `statusMessage` is a common field on hook handlers.

### post-tool-use.sh (Reminder Script)

```bash
#!/usr/bin/env bash
# PostToolUse reminder: nudge Claude to check gates when ready.
# This is reminder-only -- it does NOT run gates.

# Read the file path from stdin JSON
FILE_PATH=$(jq -r '.tool_input.file_path // empty' 2>/dev/null)

# Only remind if we got a file path
if [ -n "$FILE_PATH" ]; then
  cat <<'EOF'
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "File modified. When you're done making changes, run /assay:gate-check to verify all quality gates pass."
  }
}
EOF
fi

exit 0
```

Source: Claude Code hooks reference -- PostToolUse decision control supports `additionalContext` in `hookSpecificOutput`.

### stop-gate-check.sh (Gate Enforcement Script)

```bash
#!/usr/bin/env bash
# Stop hook: verify quality gates pass before allowing agent to complete.
# Checks stop_hook_active to prevent infinite loops.
# Checks for .assay/ directory for graceful degradation.

INPUT=$(cat)

# Prevent infinite loops: if already in a stop-hook retry, allow stop
STOP_HOOK_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

# Graceful degradation: if no .assay/ directory, allow stop
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')
if [ -z "$CWD" ] || [ ! -d "$CWD/.assay" ]; then
  exit 0
fi

# Check if assay binary exists
if ! command -v assay &>/dev/null; then
  exit 0  # No binary = can't check gates, allow stop
fi

# Run gate check (all specs)
# Use --json for structured output, capture exit code
cd "$CWD" || exit 0
GATE_OUTPUT=$(assay gate run --all --json 2>&1) || true
GATE_EXIT=$?

# If gates pass (exit 0), allow stop
if [ $GATE_EXIT -eq 0 ]; then
  exit 0
fi

# Gates failed: block the stop with reason
# Extract a concise summary from the JSON output
FAILED_COUNT=$(echo "$GATE_OUTPUT" | jq -r '.failed // 0' 2>/dev/null || echo "unknown")

cat <<EOF
{
  "decision": "block",
  "reason": "Quality gates are failing ($FAILED_COUNT criteria failed). Run /assay:gate-check for details and fix the failing criteria before completing."
}
EOF

exit 0
```

Source: Claude Code hooks reference -- Stop decision control uses top-level `decision: "block"` and `reason`. The `stop_hook_active` field is documented in Stop input schema.

**Note on `assay gate run --all`:** The current CLI does not have a `--all` flag for running all specs. The Stop hook script will need to either (a) iterate specs via `assay spec list` + loop, or (b) a new `--all` flag added to the CLI. This is flagged as an open question.

### /gate-check SKILL.md (PLG-03)

```yaml
---
name: gate-check
description: >
  Run quality gates for a spec and report pass/fail results.
  Use when checking if code changes meet spec criteria,
  after implementing features, or when asked about gate status.
---

# Gate Check

Run quality gates and report structured results.

## Steps

1. **Determine which spec(s) to check:**
   - If a spec name was provided as `$ARGUMENTS`, use that spec
   - If no spec was provided, call the `spec_list` tool to discover all available specs, then run gates for each

2. **Run gates:**
   - Call the `gate_run` tool with the spec name
   - Set `include_evidence` to `false` for the initial summary

3. **Report results:**
   - **All passed:** Report concisely: "3/3 criteria passed for [spec-name]" with duration
   - **Any failed:** List each failed criterion with its `reason` field, then offer to show full evidence by calling `gate_run` with `include_evidence: true`

4. **If multiple specs:** Report results per-spec with an aggregate summary at the end

## Output Format

Keep output concise. For passing specs, one line is enough. For failures, show the criterion name, status, and failure reason. Only show full stdout/stderr evidence when explicitly requested or when the failure reason alone is insufficient to diagnose the issue.
```

### /spec-show SKILL.md (PLG-04)

```yaml
---
name: spec-show
description: >
  Display a spec's criteria and details.
  Use when the user wants to see what a spec contains,
  what criteria need to be met, or before starting implementation.
---

# Spec Show

Display a spec's full definition including all criteria.

## Steps

1. **Determine which spec to show:**
   - If a spec name was provided as `$ARGUMENTS`, use that spec
   - If no spec was provided, call `spec_list` to show available specs and ask which one to display

2. **Fetch the spec:**
   - Call the `spec_get` tool with the spec name

3. **Present the spec:**
   - Show the spec name and description
   - List each criterion with:
     - Name
     - Description
     - Whether it's executable (has a `cmd`) or descriptive (no `cmd`)
     - The command that will be run (if executable)
     - Timeout override (if set)

## Output Format

Use a clear, structured format. Group criteria by type (executable vs descriptive). For executable criteria, show the exact command so the user knows what will run.
```

### CLAUDE.md Workflow Snippet (PLG-05)

```markdown
# Assay Workflow

This project uses Assay for spec-driven development with quality gates.

## Workflow

1. **Read the spec first.** Before writing code, always read the relevant spec:
   - Use `/assay:spec-show <spec-name>` to see all criteria
   - Understand what "done" means before starting

2. **Implement against criteria.** Each criterion in the spec defines a verifiable requirement. Write code that satisfies each one.

3. **Verify with gates.** After making changes, run the quality gates:
   - Use `/assay:gate-check <spec-name>` to run all executable criteria
   - Fix any failures before moving on

4. **Iterate until all gates pass.** Do not consider work complete until all quality gates pass.

## Commands

| Command | Description |
| --- | --- |
| `/assay:spec-show [name]` | Display a spec's criteria |
| `/assay:gate-check [name]` | Run quality gates and report results |

## MCP Tools

The Assay MCP server provides these tools directly:

| Tool | Description |
| --- | --- |
| `spec_list` | Discover available specs |
| `spec_get` | Get a spec's full definition |
| `gate_run` | Run quality gates for a spec |
```

Source: CONTEXT.md decisions -- prescriptive tone, mandatory spec-first, both abstract guidance and command reference, static default shipped with plugin.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
| --- | --- | --- | --- |
| `commands/` directory | `skills/` directory with `SKILL.md` | Claude Code ~1.0 (2025) | Skills support frontmatter, supporting files, model invocation control. `commands/` still works as legacy alias. |
| Inline hooks in settings.json | `hooks/hooks.json` in plugin + `type: "prompt"` and `type: "agent"` hooks | Claude Code ~1.0.33+ | Plugin hooks merge with user/project hooks. Prompt and agent hooks are newer alternatives to command hooks. |
| No `stop_hook_active` guard | `stop_hook_active` field in Stop input | Recent | Essential for preventing infinite loops in Stop hooks |
| No `statusMessage` field | `statusMessage` on hook handlers | Recent | Shows custom spinner text while hook runs |

**Current and stable:**
- The plugin format (`.claude-plugin/plugin.json`, skills, hooks, `.mcp.json`) is stable as of Claude Code 1.0.33+
- `${CLAUDE_PLUGIN_ROOT}` environment variable for path resolution
- Stop hook `decision: "block"` pattern for preventing completion
- PostToolUse `additionalContext` for injecting context

## Open Questions

1. **`assay gate run --all` / `assay gate run` (bare):**
   - What we know: The current CLI requires a spec name argument (`assay gate run <name>`). There is no `--all` flag or bare `assay gate run` to run all specs.
   - What's unclear: The Stop hook needs to evaluate ALL specs to determine if the agent can stop. The skill (`/gate-check`) also needs this when no spec is specified.
   - Recommendation: Either (a) add a `--all` flag to `assay gate run` in this phase, or (b) have the Stop hook script iterate specs by parsing `assay spec list` output. Option (a) is cleaner but requires a CLI change. Option (b) is possible with shell scripting but fragile. The MCP skill path already handles this through `spec_list` + loop, so the skill is fine -- only the hook script needs resolution.

2. **Configurable Stop hook enforcement:**
   - What we know: CONTEXT.md says "Stop hook defaults to hard block but is configurable -- user can soften to warn-and-allow."
   - What's unclear: How is this configured? Environment variable? Plugin config file? `.assay/config.toml` field?
   - Recommendation: Use an environment variable (`ASSAY_STOP_HOOK_MODE=enforce|warn|off`) checked by the Stop hook script. This doesn't require any Rust changes and is the simplest configuration mechanism. Default to `enforce` when unset.

3. **`assay init` enhanced CLAUDE.md generation:**
   - What we know: CONTEXT.md says "plugin ships a static default snippet; `assay init` can enhance it with project-specific details."
   - What's unclear: Whether the `assay init` enhancement is in scope for Phase 10 or deferred.
   - Recommendation: Ship the static CLAUDE.md template in the plugin now. Defer `assay init` CLAUDE.md generation to a follow-up -- it requires Rust code changes in `assay-core::init` and is a nice-to-have, not a requirement for PLG-05.

4. **Hook scope for spec selection:**
   - What we know: CONTEXT.md marks this as Claude's discretion.
   - What's unclear: Should the Stop hook check ALL specs or only specs related to the current work?
   - Recommendation: Check ALL specs. The agent doesn't have enough context in the Stop hook input to determine which specs are "current." Running all specs is the safe default. If this proves too slow, it can be refined later.

## Sources

### Primary (HIGH confidence)
- [Claude Code Plugins Reference](https://code.claude.com/docs/en/plugins-reference) -- plugin manifest schema, `.mcp.json` format, directory structure, `${CLAUDE_PLUGIN_ROOT}`, debugging
- [Claude Code Hooks Reference](https://code.claude.com/docs/en/hooks) -- complete hook event schemas, PostToolUse/Stop decision control, `stop_hook_active`, matcher patterns, JSON input/output format, exit code semantics
- [Claude Code Skills Reference](https://code.claude.com/docs/en/skills) -- SKILL.md frontmatter schema, model invocation control, `$ARGUMENTS`, supporting files, plugin namespacing
- [Claude Code Plugins Guide](https://code.claude.com/docs/en/plugins) -- plugin quickstart, migration from standalone config, testing with `--plugin-dir`

### Secondary (MEDIUM confidence)
- Existing Assay codebase analysis -- MCP server implementation (`crates/assay-mcp/src/server.rs`), CLI structure (`crates/assay-cli/src/main.rs`), existing plugin scaffolding (`plugins/claude-code/`)
- Phase 10 CONTEXT.md -- user decisions constraining hook behavior, skill output design, CLAUDE.md tone

### Tertiary (LOW confidence)
- Community plugin examples (WebSearch results) -- general patterns, not specific to Assay's use case

## Metadata

**Confidence breakdown:**
- Plugin format/structure: HIGH -- verified against official Claude Code documentation
- Skills (SKILL.md format): HIGH -- verified against official docs, straightforward markdown+frontmatter
- Hooks (PostToolUse + Stop): HIGH -- verified against official docs with exact JSON schemas
- .mcp.json format: HIGH -- verified against official docs and existing repo `.mcp.json`
- Stop hook loop prevention: HIGH -- `stop_hook_active` documented in official hooks reference
- "Run all specs" mechanism: MEDIUM -- current CLI lacks `--all` flag, needs resolution

**Research date:** 2026-03-02
**Valid until:** 2026-04-02 (stable plugin API, 30-day validity)
