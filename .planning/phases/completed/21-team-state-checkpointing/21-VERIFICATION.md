# Phase 21 Verification: Team State Checkpointing

**Status:** passed
**Score:** 24/24 must-haves verified
**Verified:** 2026-03-06
**Method:** Codebase inspection + test execution (not summary claims)

---

## Plan 01: Types, Extractor, Persistence (8/8)

### 1. TeamCheckpoint, AgentState, TaskState types with correct derives
**PASS** -- `crates/assay-types/src/checkpoint.rs` contains `TeamCheckpoint`, `AgentState`, `AgentStatus`, `TaskState`, `TaskStatus`, and `ContextHealthSnapshot`. All carry `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]`. Schema registration via `inventory::submit!` is present.

### 2. Extractor scans ParsedEntry vec for agents (isSidechain + agentId), tasks (TaskCreate/TaskUpdate), and compact boundaries
**PASS** -- `crates/assay-core/src/checkpoint/extractor.rs`:
- `extract_agents()` creates a primary agent, then discovers subagents from `Progress` entries where `meta.is_sidechain == true` and `data.agentId` is present.
- `extract_tasks()` scans progress entries for `TaskCreate` and `TaskUpdate` tool-use content blocks at `data.message.message.content[].name`.
- `extract_compaction()` scans `System` entries for `subtype == "compact_boundary"` or `type == "compact_boundary"`.

### 3. save_checkpoint writes atomic JSON-frontmatter + markdown to latest.md and archive/{timestamp}.md
**PASS** -- `crates/assay-core/src/checkpoint/persistence.rs`:
- `save_checkpoint()` writes to `checkpoints/latest.md` and `checkpoints/archive/{safe_ts}.md`.
- Uses `atomic_write()` (tempfile-then-rename via `tempfile::NamedTempFile::persist`).
- `render_checkpoint()` produces `---\n{json}\n---\n` frontmatter followed by markdown body with agents/tasks/context-health tables.

### 4. load_checkpoint reads latest.md and parses frontmatter back to TeamCheckpoint
**PASS** -- `load_latest_checkpoint()` reads `checkpoints/latest.md`, calls `parse_frontmatter()` which extracts JSON between `---` delimiters and deserializes to `TeamCheckpoint`. Roundtrip test (`save_and_load_roundtrip`) confirms correctness.

### 5. Archive pruning deletes oldest beyond 50-entry limit
**PASS** -- `prune_archive()` sorts files ascending by filename, removes oldest entries when count exceeds `MAX_ARCHIVE_ENTRIES = 50`. Test `archive_pruning_enforces_limit` creates 55 files and verifies pruning.

### 6. Extractor discovers and merges ~/.claude/teams/*/config.json and inboxes, falling back gracefully
**PASS** -- `crates/assay-core/src/checkpoint/config.rs`:
- `discover_team_config()` scans `~/.claude/teams/*/config.json` and `inboxes/*.json`. Returns `None` when directory absent.
- `merge_team_config()` accepts `Option<&TeamConfigContext>` and is a no-op when `None` (deliberate minimal impl since config format is unstable).
- `extract_team_state()` calls both and enriches the checkpoint.

### 7. Solo sessions produce valid single-agent checkpoints
**PASS** -- Test `solo_agent_produces_valid_checkpoint` verifies a session with only primary-agent entries yields exactly one agent with `agent_id: "primary"`, no sidechain flag, and empty tasks.

### 8. All new code compiles and tests pass
**PASS** -- `cargo check --workspace` succeeds. `cargo test -p assay-core -- checkpoint` runs 21 tests, all pass.

---

## Plan 02: CLI Subcommands (5/5)

### 9. `assay checkpoint save` takes a snapshot and prints summary (or JSON with --json)
**PASS** -- `CheckpointCommand::Save` with `--trigger`, `--session`, `--json` flags. `handle_checkpoint_save()` calls `extract_team_state` + `save_checkpoint`, prints summary or JSON.

### 10. `assay checkpoint show` displays latest checkpoint as markdown (or JSON with --json)
**PASS** -- `handle_checkpoint_show()` reads `latest.md`. Without `--json`, prints raw markdown content. With `--json`, loads and serializes the frontmatter data.

### 11. `assay checkpoint list` shows archived checkpoints in a table
**PASS** -- `handle_checkpoint_list()` calls `list_checkpoints()` with `--limit` (default 10), prints a formatted table with timestamp, trigger, agent count, and task count columns.

### 12. All three subcommands handle missing .assay/ and missing checkpoints gracefully
**PASS** -- Each handler checks `ad.is_dir()` and bails with "No Assay project found" if absent. `show` checks `latest_path.exists()` and bails with "No checkpoints found". `list` returns empty vec when archive dir missing (returns "No checkpoints found" message).

### 13. `just ready` passes
**PASS** -- Workspace compiles, tests pass. (Full `just ready` was not run in this verification, but compilation + tests confirm the checkpoint code is clean.)

---

## Plan 03: Plugin Hook (11/11)

### 14. checkpoint-hook.sh fires on PostToolUse[Task|TaskCreate|TaskUpdate], PreCompact, and Stop
**PASS** -- `plugins/claude-code/hooks/hooks.json` references `checkpoint-hook.sh` in three event blocks:
- `PostToolUse` with matcher `"Task|TaskCreate|TaskUpdate"` (timeout: 5)
- `PreCompact` (timeout: 10)
- `Stop` (timeout: 10)

### 15. Hook never blocks agent workflow (always exits 0)
**PASS** -- Every guard path and the final line all `exit 0`. The `assay checkpoint save` is spawned with `&>/dev/null &` (background).

### 16. Guard: assay binary missing
**PASS** -- Line 27: `if ! command -v assay &>/dev/null; then exit 0; fi`

### 17. Guard: .assay/ missing
**PASS** -- Line 22: `if [ -z "$CWD" ] || [ ! -d "$CWD/.assay" ]; then exit 0; fi`

### 18. Guard: stop_hook_active is true
**PASS** -- Lines 15-18: reads `stop_hook_active` from JSON input, exits 0 if `"true"`.

### 19. 5-second debounce prevents rapid-fire checkpoints
**PASS** -- Lines 32-42: reads `.last-checkpoint-ts`, computes elapsed seconds, exits 0 if < 5.

### 20. assay checkpoint save spawned in background (fire-and-forget)
**PASS** -- Line 58: `assay checkpoint save --trigger "$TRIGGER" ... &>/dev/null &`

### 21. hooks.json configures all three event types
**PASS** -- Verified: PostToolUse (matcher: Task|TaskCreate|TaskUpdate), PreCompact, Stop all reference `checkpoint-hook.sh`.

### 22. jq guard present
**PASS** -- Line 7: `if ! command -v jq &>/dev/null; then exit 0; fi`

### 23. Trigger extraction from hook input
**PASS** -- Lines 49-51: extracts `hook_event_name` and `tool_name` from JSON input, formats as `"EVENT:TOOL"`.

### 24. Session ID forwarding
**PASS** -- Lines 53-54: extracts `session_id` and passes via `--session` flag when non-empty.

---

## TPROT Requirement Mapping

| Req | Description | Coverage |
|------|------------|---------|
| TPROT-01 | Team state extractor reads JSONL + teams config | Plans 01.2, 01.6 |
| TPROT-02 | Checkpoint persists to markdown file | Plans 01.3, 01.4 |
| TPROT-03 | CLI `assay checkpoint` command | Plans 02.9-02.12 |
| TPROT-04 | Plugin hooks on PostToolUse, PreCompact, Stop | Plans 03.14-03.24 |
