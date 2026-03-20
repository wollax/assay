# S06: Codex Plugin — UAT

**Milestone:** M005
**Written:** 2026-03-20

## UAT Type

- UAT mode: human-experience
- Why this mode is sufficient: S06 delivers pure markdown content. Structural correctness and tool-name accuracy were verified by grep assertions (contract-level). The only remaining verification is that a human developer actually runs the skills inside Codex and finds the workflow usable. No Rust compilation or runtime assertion can substitute for this.

## Preconditions

1. Assay MCP server is running and connected to Codex (`assay-mcp` in the Codex MCP config)
2. Codex plugin is installed: `ln -s $(pwd)/plugins/codex/skills .agents/skills/assay` (or equivalent path)
3. A `.assay/milestones/` directory exists with at least one milestone file, OR the tester is prepared to create one via `/assay:plan`
4. At least one spec exists under `.assay/specs/` with a `gates.toml`

## Smoke Test

Run `/assay:gate-check` in Codex. It should list available specs and offer to run gates on one or all of them. If it prompts for a spec slug and returns gate results, the plugin is wired correctly.

## Test Cases

### 1. Gate check on a real spec

1. In Codex, invoke `/assay:gate-check`
2. When prompted, provide a spec slug from `.assay/specs/`
3. **Expected:** Skill calls `spec_list` then `gate_run` on the selected spec; outputs pass/fail table with gate names and results

### 2. Spec show — view criteria for a spec

1. In Codex, invoke `/assay:spec-show`
2. When prompted, provide a spec slug
3. **Expected:** Skill calls `spec_list` then `spec_get`; outputs spec name, description, and all criteria with descriptions

### 3. Cycle status — active milestone overview

1. In Codex, invoke `/assay:cycle-status`
2. **Expected (no active milestone):** Skill returns a message indicating no active milestone and suggests running `/assay:plan`
3. **Expected (active milestone):** Skill calls `cycle_status` then `chunk_status`; outputs milestone name, phase, chunk progress (X/N), active chunk slug, and latest gate pass/fail counts

### 4. Next chunk — active chunk detail

1. In Codex, invoke `/assay:next-chunk`
2. **Expected (no active milestone):** Skill handles `{"active":false}` gracefully — returns guidance to run `/assay:plan`, does not error
3. **Expected (active milestone):** Skill calls `cycle_status`, `chunk_status`, and `spec_get`; outputs chunk slug/name, gate status, and full criteria list grouped by executable vs descriptive

### 5. Plan — create a new milestone interactively

1. In Codex, invoke `/assay:plan`
2. Answer all three interview questions:
   - Milestone goal/name/slug
   - Chunk list (1–3 chunks with slugs and names)
   - Success criteria per chunk (text descriptions)
3. **Expected:** Skill does NOT call any MCP tool until all three inputs are collected; then calls `milestone_list` for slug check, then `milestone_create`, then `spec_create` for each chunk
4. **Expected output:** Confirmation of milestone and spec file paths created; warning that `cmd` fields in `gates.toml` must be manually edited before gates are runnable

## Edge Cases

### cycle-status with a new chunk (no gate history)

1. Activate a milestone with a chunk that has never had gates run
2. Invoke `/assay:cycle-status`
3. **Expected:** Skill handles `{"has_history":false}` gracefully — shows "no gate history yet" rather than erroring or showing 0/0 counts ambiguously

### next-chunk with a new chunk (no gate history)

1. Same precondition as above
2. Invoke `/assay:next-chunk`
3. **Expected:** Skill shows the full criteria list from `spec_get` but explicitly notes no gate runs have occurred yet

### plan — slug collision detection

1. Create a milestone slug that already exists in `.assay/milestones/`
2. Invoke `/assay:plan` and enter that same slug when prompted
3. **Expected:** Skill calls `milestone_list`, detects the collision, and asks the user to choose a different slug before proceeding

## Failure Signals

- Skill invokes an MCP tool but returns "tool not found" → MCP server not connected or tool name mismatch
- `/assay:cycle-status` errors instead of returning guidance when no milestone is active → active:false handling not working
- `/assay:plan` calls `milestone_create` before collecting all three inputs → interview-first ordering broken
- `AGENTS.md` is not auto-loaded by Codex → verify symlink installation and that Codex reads `.agents/skills/assay/` directory
- Gate results show wrong tool name errors (e.g., `gate_check` instead of `gate_run`) → verify skill file content with `cat plugins/codex/skills/gate-check.md`

## Requirements Proved By This UAT

- R048 — Codex plugin AGENTS.md + 5 skills functional end-to-end inside real Codex environment; workflow guide gives complete overview; skills cover the full development cycle (gate-check, spec-show, cycle-status, next-chunk, plan)

## Not Proven By This UAT

- MCP tool correctness (tool signatures, parameter names, response schemas) — these are proven by S01–S03 unit tests, not by this UAT
- Gate evaluation accuracy — proven by existing gate tests in assay-core
- Milestone/cycle state transitions — proven by S02 tests
- PR creation workflow — covered by S04 UAT; not exposed in Codex plugin
- Claude Code plugin parity — covered by S05 UAT separately

## Notes for Tester

- The `cmd` fields in generated `gates.toml` files are intentionally left as placeholder text after `/assay:plan` — this is a known limitation (D076). You must manually edit these fields before gates are runnable. The plan skill warns about this explicitly.
- AGENTS.md is auto-loaded as agent instructions by Codex when it scans `.agents/skills/assay/`. If it is not loading, check the symlink points to `plugins/codex/skills/` (the directory containing the skill `.md` files and AGENTS.md).
- Skills are flat `.md` files, not subdirectory SKILL.md. If Codex expects a different discovery pattern, the symlink target may need adjustment.
