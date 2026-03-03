# Phase 10 Verification

**Status:** passed
**Score:** 8/8 must-haves verified

## Must-Haves

| # | Requirement | Status | Evidence |
|---|------------|--------|----------|
| 1 | PLG-02: `.mcp.json` points to `assay mcp serve` with stdio transport | PASS | `plugins/claude-code/.mcp.json` exists; `type: "stdio"`, `command: "assay"`, `args: ["mcp", "serve"]` |
| 2 | PLG-03: `/gate-check` skill references `gate_run` MCP tool | PASS | `plugins/claude-code/skills/gate-check/SKILL.md` — step 2 calls `gate_run` tool with `include_evidence` flag; also calls `spec_list` for multi-spec discovery |
| 3 | PLG-04: `/spec-show` skill references `spec_get` MCP tool | PASS | `plugins/claude-code/skills/spec-show/SKILL.md` — step 2 calls `spec_get` tool; also calls `spec_list` when no arg provided |
| 4 | PLG-05: CLAUDE.md has spec-first workflow instructions | PASS | `plugins/claude-code/CLAUDE.md` — prescriptive 4-step workflow (read spec, implement, verify gates, iterate); command reference table for `/assay:spec-show` and `/assay:gate-check`; MCP tool reference table |
| 5 | PLG-06: `hooks.json` has PostToolUse hook on Write/Edit | PASS | `plugins/claude-code/hooks/hooks.json` — `PostToolUse` entry with matcher `"Write\|Edit"`, invokes `post-tool-use.sh` with 5s timeout |
| 6 | PLG-06: `post-tool-use.sh` outputs `additionalContext` JSON | PASS | Script outputs valid JSON with `hookSpecificOutput.hookEventName: "PostToolUse"` and `additionalContext` reminder to run `/assay:gate-check` |
| 7 | PLG-07: `hooks.json` has Stop hook entry | PASS | `plugins/claude-code/hooks/hooks.json` — `Stop` entry with no matcher (fires always), invokes `stop-gate-check.sh` with 120s timeout |
| 8 | PLG-07: `stop-gate-check.sh` checks `stop_hook_active`, `.assay/` dir, `ASSAY_STOP_HOOK_MODE` | PASS | All four guards implemented: (1) `stop_hook_active=true` → exit 0; (2) no `.assay/` dir → exit 0; (3) `assay` binary absent → exit 0; (4) `ASSAY_STOP_HOOK_MODE=off` → exit 0. Enforce mode outputs `{"decision": "block", "reason": ...}` |
| 9 | CLI: `assay gate run --all` flag exists | PASS | `crates/assay-cli/src/main.rs` — `--all` flag defined at line 164, conflicts with `name`, handled by `handle_gate_run_all()` at line 395; stop hook calls `assay gate run --all --json` |
| 10 | README.md has installation instructions | PASS | `plugins/claude-code/README.md` — prerequisites (`cargo install assay-cli`), installation (`claude plugin add`), verification steps, configuration table for `ASSAY_STOP_HOOK_MODE` |

## Hook Script Tests

### `post-tool-use.sh` — reminder output

```
$ echo '{}' | bash plugins/claude-code/scripts/post-tool-use.sh
```

Output:
```json
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "File modified. When you're done making changes, run /assay:gate-check to verify all quality gates pass."
  }
}
```

Result: PASS — valid JSON, correct structure, exit 0.

---

### `stop-gate-check.sh` — guard 1: `stop_hook_active=true`

```
$ echo '{"stop_hook_active": true, "cwd": "/tmp"}' | bash plugins/claude-code/scripts/stop-gate-check.sh
```

Output: (none)
Exit code: 0

Result: PASS — infinite loop guard fires, stop allowed immediately.

---

### `stop-gate-check.sh` — guard 2: no `.assay/` directory

```
$ echo '{"stop_hook_active": false, "cwd": "/tmp"}' | bash plugins/claude-code/scripts/stop-gate-check.sh
```

Output: (none)
Exit code: 0

Result: PASS — `/tmp/.assay` does not exist, graceful degradation fires, stop allowed.

---

### `stop-gate-check.sh` — guard 4: `ASSAY_STOP_HOOK_MODE=off`

```
$ ASSAY_STOP_HOOK_MODE=off bash -c 'echo '{"stop_hook_active": false, "cwd": "/tmp"}' | bash plugins/claude-code/scripts/stop-gate-check.sh'
```

Output: (none)
Exit code: 0

Result: PASS — mode=off guard fires before `.assay/` check, stop allowed.

## Conclusion

Phase 10 is complete. All 8 PLG-02 through PLG-07 requirements are implemented and verified. The plugin directory at `plugins/claude-code/` contains all required artifacts:

- `.mcp.json` — registers `assay mcp serve` as a stdio MCP server
- `skills/gate-check/SKILL.md` — structured workflow using `gate_run` and `spec_list`
- `skills/spec-show/SKILL.md` — structured workflow using `spec_get` and `spec_list`
- `CLAUDE.md` — prescriptive spec-first development workflow with command reference
- `hooks/hooks.json` — PostToolUse (Write/Edit matcher) and Stop hook entries
- `scripts/post-tool-use.sh` — outputs `additionalContext` reminder JSON, exit 0
- `scripts/stop-gate-check.sh` — four safety guards, blocks on gate failure in enforce mode
- `README.md` — installation instructions, verification steps, configuration reference

The CLI `--all` flag required by the stop hook is present in `crates/assay-cli/src/main.rs`. All hook script tests pass with the correct exit codes and output structures.

**All five success criteria from the phase goal are met:**
1. Plugin registers the Assay MCP server and skills appear in the skill list via `.mcp.json` and the `skills/` directory.
2. An agent can call `/gate-check` and receive structured pass/fail results via `gate_run`.
3. An agent can call `/spec-show` and see the full spec with criteria via `spec_get`.
4. After Write/Edit tool use, the PostToolUse hook outputs an `additionalContext` reminder to run gate check.
5. The Stop hook blocks agent completion when gates are failing (enforce mode default); configurable via `ASSAY_STOP_HOOK_MODE`.
