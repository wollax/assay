---
phase: 10
plan: "10-01"
subsystem: plugin
tags: [claude-code, mcp, skills, plugin-config]
dependency-graph:
  requires: [phase-8, phase-9]
  provides: [mcp-registration, gate-check-skill, spec-show-skill, workflow-snippet]
  affects: [10-02]
tech-stack:
  added: []
  patterns: [skills-as-mcp-orchestrators, yaml-frontmatter-skills]
key-files:
  created:
    - plugins/claude-code/.mcp.json
    - plugins/claude-code/skills/gate-check/SKILL.md
    - plugins/claude-code/skills/spec-show/SKILL.md
    - plugins/claude-code/CLAUDE.md
  modified:
    - plugins/claude-code/README.md
    - .gitignore
decisions:
  - ".gitignore negation pattern for plugin .mcp.json distribution (root .mcp.json stays ignored)"
  - "Skills use MCP tool orchestration pattern (not CLI invocation via Bash)"
  - "CLAUDE.md uses prescriptive tone with 4-step spec-first workflow"
  - "README documents ASSAY_STOP_HOOK_MODE env var for Stop hook configuration"
metrics:
  duration: "2m 29s"
  completed: "2026-03-03"
---

# Phase 10 Plan 01: MCP Config and Skill Definitions Summary

Static plugin configuration delivering MCP server registration, /gate-check and /spec-show skills as MCP tool orchestrators, prescriptive CLAUDE.md workflow snippet, and comprehensive README with installation and verification docs.

## What Was Done

### Task 1: MCP config and skill definitions

Created `.mcp.json` registering the Assay MCP server with stdio transport and PATH-based binary lookup (`"command": "assay"`, `"args": ["mcp", "serve"]`).

Created `/gate-check` SKILL.md with YAML frontmatter instructing Claude to:
- Discover specs via `spec_list` when no argument provided
- Run gates via `gate_run` with `include_evidence: false` for initial summary
- Report concisely on pass, detailed on failure with evidence retrieval

Created `/spec-show` SKILL.md with YAML frontmatter instructing Claude to:
- Discover specs via `spec_list` when no argument provided
- Fetch full spec via `spec_get`
- Present criteria grouped by type (executable vs descriptive)

Removed `skills/.gitkeep` since real skill directories now exist.

Added `.gitignore` negation pattern (`!plugins/**/.mcp.json`) so the plugin's `.mcp.json` is tracked while the root-level dev `.mcp.json` remains ignored.

### Task 2: CLAUDE.md workflow snippet and README update

Created `CLAUDE.md` as the workflow snippet template with prescriptive spec-first development process:
1. Read the spec first (`/assay:spec-show`)
2. Implement against criteria
3. Verify with gates (`/assay:gate-check`)
4. Iterate until all gates pass

Includes both a Commands table (skill references) and an MCP Tools table (direct tool references).

Updated `README.md` with:
- Plugin description matching plugin.json
- Prerequisites (binary in PATH, install via `cargo install assay-cli`)
- Installation (`claude plugin add` and `--plugin-dir` for dev)
- Verification checklist (MCP tools and skills)
- What the plugin provides (MCP server, skills, hooks, CLAUDE.md)
- Configuration (`ASSAY_STOP_HOOK_MODE` env var with enforce/warn/off modes)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] .gitignore blocks plugin .mcp.json**

- **Found during:** Task 1
- **Issue:** Root `.gitignore` has `.mcp.json` pattern that blocks tracking the plugin's `.mcp.json` which needs to be distributed
- **Fix:** Added negation pattern `!plugins/**/.mcp.json` to `.gitignore` and used `git add -f` for initial tracking
- **Files modified:** `.gitignore`
- **Commit:** 5de0227

## Decisions Made

| Decision | Rationale |
| --- | --- |
| .gitignore negation for plugin .mcp.json | Root `.mcp.json` ignore is for local dev convenience; plugin `.mcp.json` must be distributed |
| Skills as MCP tool orchestrators | Per research: skills instruct Claude to call MCP tools, not shell commands |
| Prescriptive CLAUDE.md tone | Per CONTEXT.md: mandatory spec-first, step-by-step workflow |

## Verification Results

- All 5 plugin files exist and are non-empty
- `.mcp.json` validates as valid JSON
- Skills have correct YAML frontmatter names
- Skills reference correct MCP tools (gate_run, spec_get, spec_list)
- CLAUDE.md references both skills and MCP tools
- README has installation and binary install instructions
- `just ready` passes cleanly (no Rust changes in this plan)

## Next Phase Readiness

Plan 10-02 (hooks and scripts) can proceed. The `.mcp.json`, skills, CLAUDE.md, and README are all in place. Hooks configuration (`hooks.json`) and shell scripts for PostToolUse/Stop are the remaining deliverables.
