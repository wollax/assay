# S05: Claude Code Plugin Upgrade — Research

**Date:** 2026-03-20

## Summary

S05 upgrades the Claude Code plugin from a gate-runner integration to a full milestone-aware development cycle surface. All 8 new MCP tools from S01–S04 are already registered and tested — no MCP changes are needed. The deliverables are entirely in the `plugins/claude-code/` directory plus one small Rust CLI addition: a `--json` flag on `assay milestone status` to give the new bash hooks a machine-readable way to detect the active chunk.

The skill files (`plan.md`, `status.md`, `next-chunk.md`) follow the existing subdirectory pattern (`skills/<name>/SKILL.md`). The cycle-stop-check script replaces `stop-gate-check.sh` and delegates to `assay gate run <active-chunk-slug> --json` when a milestone is active, falling back to `--all` when not. The `post-tool-use.sh` is updated to mention the active chunk name in its reminder.

The only Rust work is adding `--json` to `assay milestone status` — the hook needs the active chunk slug, and parsing the text output of `assay milestone status` in bash is fragile. Adding `--json` outputs `CycleStatus` JSON (already derived `Serialize`) when a milestone is active, or `{"active": false}` when not — matching the MCP `cycle_status` response shape exactly.

## Recommendation

Implement S05 in three tasks:
1. **T01 — Rust: add `--json` to `assay milestone status`** — small CLI change, enables all hooks.
2. **T02 — Skill files + CLAUDE.md** — pure markdown, no tests needed (content verification only).
3. **T03 — Hook scripts + hooks.json** — bash scripts and JSON wiring; test via `just ready` (no new Rust tests since hooks are not Rust).

All three are independent of each other after T01 completes (T02/T03 can be parallelised if desired).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Detect active milestone/chunk in bash | `assay milestone status --json` (add this flag in T01) | Avoids fragile TOML parsing in bash; outputs CycleStatus JSON already used by `cycle_status` MCP tool |
| Run gates for a specific chunk | `assay gate run <slug> --json` (already exists) | Clean JSON output; exactly what the stop hook needs to scope to active chunk |
| Run gates for all specs (fallback) | `assay gate run --all --json` (already exists) | Current stop-gate-check.sh already uses this; keep as fallback |
| Hook infinite-loop prevention | `stop_hook_active` field in hook input JSON | Existing pattern in `stop-gate-check.sh` — copy exactly |

## Existing Code and Patterns

- `plugins/claude-code/scripts/stop-gate-check.sh` — the cycle-stop-check.sh replaces this; copy all 5 safety guards verbatim (jq, stop_hook_active, MODE, `.assay/`, binary check); add cycle-aware logic between guard 4 and guard 5
- `plugins/claude-code/scripts/checkpoint-hook.sh` — PreCompact/Stop checkpoint hook; **do not change**, leave as-is per S05-CONTEXT.md (the milestone TOML is already persisted)
- `plugins/claude-code/scripts/post-tool-use.sh` — update to call `assay milestone status --json` and include active chunk name when available; must exit 0 and never block
- `plugins/claude-code/hooks/hooks.json` — update `Stop[0]` command from `stop-gate-check.sh` to `cycle-stop-check.sh`; all other hooks stay unchanged
- `plugins/claude-code/skills/gate-check/SKILL.md` — reference for skill markdown format: frontmatter + `## Steps` + `## Output Format`; new skills follow the same pattern
- `plugins/claude-code/skills/spec-show/SKILL.md` — second reference; note the `$ARGUMENTS` convention for skill invocation
- `crates/assay-cli/src/commands/milestone.rs` — `milestone_status_cmd()` at line 59; add `#[arg(long)] json: bool` to `MilestoneCommand::Status` variant; in handler: if json → call `assay_core::milestone::cycle_status(&dir)` and serialize to JSON
- `crates/assay-core/src/milestone/mod.rs` — re-exports `CycleStatus` and `cycle_status` (line 3-5); accessible as `assay_core::milestone::cycle_status` and `assay_core::milestone::CycleStatus` from assay-cli
- `crates/assay-mcp/src/server.rs` — `cycle_status` at line 3402 returns `{"active":false}` on no active milestone; **match this shape** in `milestone status --json` for consistency

## Constraints

- **No MCP changes** — all 8 new tools (`milestone_list`, `milestone_get`, `cycle_status`, `cycle_advance`, `chunk_status`, `milestone_create`, `spec_create`, `pr_create`) are already registered; S05 only adds consumers (skills, hooks)
- **CLAUDE.md must stay concise** — injected into every conversation; a command/skill table plus a short workflow paragraph maximum; detailed instructions go in skill files, not CLAUDE.md
- **Hooks must never block** — all hooks except the Stop hook must always exit 0; the Stop hook may output `{ decision: "block", reason: "..." }` JSON only when in enforce mode and gates fail
- **Stop hook fallback** — when no milestone is in_progress, fall back to `assay gate run --all --json` (exact current behavior); backward-compatible with non-milestone projects
- **Skill subdirectory pattern** — new skills go in `plugins/claude-code/skills/<name>/SKILL.md` (not flat `.md` files); matches existing `gate-check/` and `spec-show/` structure
- **GatesSpec struct literals** — when adding `--json` CLI flag, there are no new struct literal changes; the flag is on a CLI enum variant, not a type shared across the workspace
- **Just ready must stay green** — 1332 existing tests; `--json` flag addition requires a test in `assay-cli` following the existing `assert!(result.is_ok()` pattern; one test for `milestone_status_json_no_active` (exits 0, outputs `{"active":false}`)

## MCP Tool Reference for Skills

All tools called by S05 skills; every tool is already registered and tested:

| Tool | Params | Used by |
|------|--------|---------|
| `milestone_create` | `{slug, name, description?, chunks: [{slug, name}]}` | `/assay:plan` |
| `spec_create` | `{slug, name, description?, milestone_slug?, criteria: [strings]}` | `/assay:plan` |
| `cycle_status` | `{}` | `/assay:status`, `/assay:next-chunk` |
| `chunk_status` | `{chunk_slug}` | `/assay:next-chunk` |
| `spec_get` | `{name}` | `/assay:next-chunk` (load active chunk spec) |
| `cycle_advance` | `{milestone_slug?}` | Referenced in CLAUDE.md |
| `pr_create` | `{milestone_slug, title, body?}` | Referenced in CLAUDE.md |

`cycle_status` returns `{"active": false}` when no milestone is in_progress, otherwise returns `CycleStatus`:
```json
{
  "milestone_slug": "my-feature",
  "milestone_name": "My Feature",
  "phase": "InProgress",
  "active_chunk_slug": "chunk-2",
  "completed_count": 1,
  "total_count": 3
}
```

## CLI Commands for CLAUDE.md

Full skill/command surface to document:

| Surface | What it does |
|---------|-------------|
| `/assay:plan` | Interview user → call `milestone_create` + `spec_create` per chunk |
| `/assay:status` | Call `cycle_status` → show milestone/chunk/phase progress |
| `/assay:next-chunk` | Call `cycle_status` + `chunk_status` + `spec_get` → show active chunk context |
| `/assay:spec-show [name]` | Show spec criteria (existing) |
| `/assay:gate-check [name]` | Run gates (existing) |
| `assay plan` | Interactive CLI wizard (non-TTY guard) |
| `assay milestone list` | List all milestones |
| `assay milestone status` | Show in_progress milestone progress |
| `assay milestone advance` | Evaluate gates + mark active chunk complete |
| `assay pr create <slug>` | Gate-gated PR creation via `gh` |

## Common Pitfalls

- **`cycle_status` returns `{"active": false}` not `null`** — the MCP tool was designed to return this sentinel; the bash hook must check `.active == false` (via `jq .active`) not check for null. The `assay milestone status --json` implementation should match: output `{"active":false}` when no milestone is in_progress.
- **`active_chunk_slug` can be null in CycleStatus** — when all chunks are complete but milestone hasn't transitioned to Verify yet; the stop hook must handle `active_chunk_slug == null` by falling back to `--all`.
- **Skill file naming** — new skills must be at `skills/<name>/SKILL.md` where `<name>` is the slug used in `/assay:<name>` invocation. Plan/status/next-chunk → `skills/plan/SKILL.md`, `skills/status/SKILL.md`, `skills/next-chunk/SKILL.md`.
- **`MilestoneStatus` serializes as a Rust enum variant name** — values are `"Draft"`, `"InProgress"`, `"Verify"`, `"Complete"` (PascalCase) not lowercase. Bash comparison: `jq -r '.phase == "InProgress"'`.
- **T01 test must handle `--features assay-types/orchestrate` workaround** — existing CLI tests run fine; add the new test to `crates/assay-cli/src/commands/milestone.rs` following the existing `assert!(result.is_ok()` pattern; the feature workaround is only needed for `-p assay-core` standalone tests, not `-p assay-cli`.
- **Plugin version** — bump `plugins/claude-code/.claude-plugin/plugin.json` version from `0.4.0` to `0.5.0` to signal the upgrade.

## Open Risks

- **`assay milestone status --json` shape** — must exactly match the `{"active": false}` sentinel from the MCP `cycle_status` tool. If the CLI uses a different sentinel (e.g., empty string or exit code), the stop hook bash logic becomes more complex. Recommendation: use identical JSON shape.
- **Slow stop hook** — if the active chunk has many long-running gate criteria, the Stop hook can timeout (current `timeout: 120`). The cycle-stop-check scoping to `active-chunk-slug` (instead of `--all`) mitigates this. Document in README that `ASSAY_STOP_HOOK_MODE=warn` is recommended for large gate suites.
- **`/assay:plan` skill scope** — the plan skill must interview the user before calling MCP tools (D066 wizard UX intent). If the skill is invoked without context, it should ask questions, not call `milestone_create` immediately. Skill instructions must be explicit about the interview step.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Bash scripting | — | none found (not needed — patterns already established in existing scripts) |
| Claude Code hooks | — | none found (documented inline in existing scripts) |

## Sources

- S05-CONTEXT.md: scope, constraints, integration points, open questions — authoritative
- `plugins/claude-code/scripts/stop-gate-check.sh` — complete hook contract (guards, JSON output protocol)
- `crates/assay-cli/src/commands/milestone.rs` — `milestone_status_cmd` implementation for T01
- `crates/assay-core/src/milestone/cycle.rs` — `CycleStatus` struct (Serialize, fields); `cycle_status()` function signature
- `crates/assay-mcp/src/server.rs` lines 3396–3520 — `cycle_status` MCP tool (sentinel shape), `chunk_status`, `cycle_advance`, `pr_create`, `milestone_create`, `spec_create` param structs and tool descriptions
- S01–S04 SUMMARY files — confirmed all 8 MCP tools registered and working; 1332 workspace tests passing after S04
