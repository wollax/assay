---
id: T03
parent: S05
milestone: M005
provides:
  - plugins/claude-code/scripts/cycle-stop-check.sh — cycle-aware Stop hook scoping gate evaluation to active chunk
  - plugins/claude-code/scripts/post-tool-use.sh — updated with active chunk name injection in reminder message
  - plugins/claude-code/hooks/hooks.json — Stop[0] wired to cycle-stop-check.sh
  - plugins/claude-code/.claude-plugin/plugin.json — version bumped to 0.5.0
  - Cargo.toml workspace version bumped to 0.5.0 (required by check-plugin-version guard)
key_files:
  - plugins/claude-code/scripts/cycle-stop-check.sh
  - plugins/claude-code/scripts/post-tool-use.sh
  - plugins/claude-code/hooks/hooks.json
  - plugins/claude-code/.claude-plugin/plugin.json
  - Cargo.toml
key_decisions:
  - Workspace Cargo.toml version bumped to 0.5.0 alongside plugin.json — the `check-plugin-version` just recipe enforces exact match between workspace version and plugin.json version; bumping only plugin.json causes `just ready` to fail
  - Active milestone detection uses `jq 'has("milestone_slug")'` rather than checking `.active` key — `{"active":false}` response from T01 does not include `.active` key explicitly; milestone_slug presence is the correct sentinel (per S05-RESEARCH.md Common Pitfalls)
  - post-tool-use.sh switched from heredoc to `jq -n --arg` for message construction — heredoc can't interpolate `$ACTIVE_CHUNK_MSG` shell variable; jq --arg passes the value safely
patterns_established:
  - Cycle-aware hook pattern: call `assay milestone status --json`, detect active milestone via `jq 'has("milestone_slug")'`, extract `active_chunk_slug`, scope gate run accordingly with fallback to `--all`
  - Shell message interpolation in Claude hook JSON output: use `jq -n --arg msg "$VAR"` rather than heredoc when the message contains a dynamic shell variable
observability_surfaces:
  - Stop hook outputs `{ decision: "block", reason: "Quality gates failing for chunk '<slug>' (N criteria). Run /assay:gate-check <slug> for details." }` when scoped to active chunk in enforce mode
  - Stop hook outputs `{ systemMessage: "Warning: quality gates are failing for chunk '<slug>' (N criteria)..." }` in warn mode
  - PostToolUse reminder now appends " Active chunk: <slug>." when a milestone is in_progress — visible in Claude's context after each file write/edit
  - Runtime inspection: `assay milestone status --json | jq .active_chunk_slug` — shows what the hooks will scope to
duration: ~30 min
verification_result: passed
completed_at: 2026-03-20
blocker_discovered: false
---

# T03: Write cycle-stop-check.sh, update post-tool-use.sh, hooks.json, and plugin version

**Created cycle-aware Stop hook that scopes gate evaluation to the active chunk slug, updated the PostToolUse reminder to name the active chunk, wired hooks.json, and bumped plugin + workspace version to 0.5.0.**

## What Happened

Created `plugins/claude-code/scripts/cycle-stop-check.sh` by combining the five verbatim guards from `stop-gate-check.sh` with a new cycle-detection block between guards 4 and 5. The detection block calls `assay milestone status --json` and uses `jq 'has("milestone_slug")'` to determine if a milestone is active (the sentinel is presence of `milestone_slug`, not a boolean `.active` field, per S05-RESEARCH.md guidance). When `active_chunk_slug` is non-null, the hook runs `assay gate run "$ACTIVE_CHUNK" --json`; otherwise it falls back to `assay gate run --all --json`. Enforce and warn mode output messages include the chunk slug for clarity.

Updated `post-tool-use.sh` to call `assay milestone status --json` after consuming stdin, check for an active chunk, and append " Active chunk: $CHUNK." to the additionalContext string. Switched from heredoc to `jq -n --arg msg` for the JSON output so the interpolated variable is passed correctly.

Updated `hooks.json` Stop[0] command from `stop-gate-check.sh` to `cycle-stop-check.sh`. Bumped `plugin.json` to `0.5.0`. Discovered that the `check-plugin-version` just recipe enforces plugin.json == workspace Cargo.toml version, so also bumped workspace version to `0.5.0` to keep `just ready` green.

## Verification

- `bash -n plugins/claude-code/scripts/cycle-stop-check.sh` — no syntax errors ✓
- `bash -n plugins/claude-code/scripts/post-tool-use.sh` — no syntax errors ✓
- `grep "cycle-stop-check.sh" plugins/claude-code/hooks/hooks.json` — exits 0 ✓
- `grep "stop-gate-check.sh" plugins/claude-code/hooks/hooks.json` — exits non-zero (old reference removed) ✓
- `grep '"version": "0.5.0"' plugins/claude-code/.claude-plugin/plugin.json` — exits 0 ✓
- `just ready` — "All checks passed." (1331 tests, 0 failures) ✓

## Diagnostics

- Active chunk at runtime: `assay milestone status --json | jq -r '.active_chunk_slug // "none"'`
- Stop hook scoping: `cat plugins/claude-code/scripts/cycle-stop-check.sh` — shows guard logic and branch
- Cycle state: `assay milestone status --json | jq .` — what the hook sees
- Degrade gracefully: if `assay` is not on PATH, all guards exit 0 (allow stop)

## Deviations

- **Workspace version bumped to 0.5.0** (not in T03-PLAN.md): the `check-plugin-version` just recipe enforces plugin.json == workspace version, making `just ready` fail if only plugin.json is bumped. Bumping workspace version is the correct fix per the project's versioning convention (`just sync-plugin-version` is the canonical sync mechanism).
- **post-tool-use.sh uses `jq -n --arg` instead of heredoc**: heredoc cannot interpolate `$ACTIVE_CHUNK_MSG` — the variable would be emitted literally. Using `jq --arg` is the correct pattern for dynamic JSON construction in bash; the output shape is identical.

## Known Issues

None.

## Files Created/Modified

- `plugins/claude-code/scripts/cycle-stop-check.sh` — new cycle-aware Stop hook (created, made executable)
- `plugins/claude-code/scripts/post-tool-use.sh` — updated with active chunk name injection
- `plugins/claude-code/hooks/hooks.json` — Stop[0] command updated to cycle-stop-check.sh
- `plugins/claude-code/.claude-plugin/plugin.json` — version bumped to 0.5.0
- `Cargo.toml` — workspace version bumped to 0.5.0 (required by check-plugin-version)
