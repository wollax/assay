---
id: S05
milestone: M005
status: ready
---

# S05: Claude Code Plugin Upgrade — Context

## Goal

Upgrade the Claude Code plugin from a gate-runner integration to a full milestone-aware development cycle surface — `/assay:plan`, `/assay:status`, `/assay:next-chunk` skills, cycle-aware hooks, and an updated CLAUDE.md.

## Why this Slice

S01–S04 built the milestone types, cycle state machine, wizard authoring, and PR workflow — but the Claude Code plugin still only knows about specs and gates. Without S05, Claude Code users cannot access the new development cycle from within their agent workflow. S05 makes the full plan→execute→verify→PR loop usable inside Claude Code. S06 (Codex) depends on the same skill patterns established here.

## Scope

### In Scope

- 3 new skills: `plan.md`, `status.md`, `next-chunk.md`
- Updated `CLAUDE.md` — concise command reference + 1-paragraph workflow summary (not a full step-by-step guide); skills contain the detailed instructions
- Replace the existing `stop-gate-check.sh` with a cycle-aware Stop hook (`cycle-stop-check.sh`) that checks incomplete chunks rather than raw gates — the new hook subsumes the old one
- Update `post-tool-use.sh` to be cycle-aware — mention the active chunk name in the reminder when working in a milestone context
- Update `hooks.json` to wire the new/replaced hooks
- Drop the `milestone-checkpoint.sh` PreCompact hook from the boundary map — the existing `checkpoint-hook.sh` is sufficient since cycle state lives in the milestone TOML which is already persisted

### Out of Scope

- No changes to `.mcp.json` — the Assay MCP server already exposes all 8 new tools; the plugin just consumes them via skills
- No new agents (the `agents/` directory stays empty)
- No new commands (the `commands/` directory stays empty)
- TUI integration (M006)
- PR advanced workflow (labels, reviewers, templates) — M008
- Codex plugin (S06)

## Constraints

- The `/assay:plan` skill instructs the agent to **interview the user** about goal, chunk breakdown, and criteria before calling `milestone_create` and `spec_create` MCP tools — mirroring the interactive CLI wizard experience, not bypassing the user
- `/assay:next-chunk` loads only the active chunk's spec and gate status — no prior completed chunk summaries. Keep it focused; the user can ask for more context if needed
- `CLAUDE.md` stays concise — a command/skill table plus a short workflow paragraph. The skills themselves contain detailed instructions. CLAUDE.md is injected into every conversation and should not bloat context
- The cycle-stop-check must gracefully degrade when no milestone is active (fall back to the existing gate-check behavior or simply allow stop)
- The post-tool-use hook must gracefully degrade when no milestone is active (fall back to the existing generic reminder)
- All existing hooks (checkpoint on PreCompact/Stop/PostToolUse[Task]) remain unchanged

## Integration Points

### Consumes

- `milestone_create` MCP tool — `/assay:plan` skill calls this after collecting user input
- `spec_create` MCP tool — `/assay:plan` skill calls this per chunk after milestone creation
- `cycle_status` MCP tool — `/assay:status` and `/assay:next-chunk` skills call this
- `chunk_status` MCP tool — `/assay:next-chunk` skill calls this for gate status
- `cycle_advance` MCP tool — referenced in CLAUDE.md workflow but not directly called by skills (agent calls it when ready)
- `pr_create` MCP tool — referenced in CLAUDE.md workflow for PR creation step
- `spec_get` MCP tool — `/assay:next-chunk` uses this to load the active chunk's spec
- `gate_run` MCP tool — cycle-stop-check hook calls `assay gate run` for the active chunk

### Produces

- `plugins/claude-code/skills/plan.md` — guided milestone authoring skill
- `plugins/claude-code/skills/status.md` — cycle progress display skill
- `plugins/claude-code/skills/next-chunk.md` — active chunk context loading skill
- `plugins/claude-code/CLAUDE.md` — updated workflow guide with milestone commands
- `plugins/claude-code/scripts/cycle-stop-check.sh` — replaces `stop-gate-check.sh`
- `plugins/claude-code/scripts/post-tool-use.sh` — updated with cycle-aware reminder
- `plugins/claude-code/hooks/hooks.json` — updated to wire new Stop hook

## Open Questions

- **Cycle-stop-check fallback behavior**: When no milestone is active, should the stop hook fall back to checking all specs (current behavior) or just allow stop? Current thinking: fall back to checking all specs — preserves backward compatibility for non-milestone projects.
- **PostToolUse chunk detection**: The post-tool-use hook needs to detect the active chunk name. Should it call `assay milestone status` (subprocess) or parse milestone TOML directly? Current thinking: call `assay milestone status --json` if it exists, or a lightweight CLI command — avoid parsing TOML in bash.
