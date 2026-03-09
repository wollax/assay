# Features Research -- v0.3.0 (Worktrees, Headless Agents, Session Records, Independent Evaluation, TUI Viewer)

**Research Date:** 2026-03-08
**Scope:** How do worktree lifecycle management, headless agent launching, session record tracking, independent gate evaluation with diff context, and minimal TUI viewers work in comparable tools?
**Existing Base:** v0.2.0 ships dual-track quality gates (deterministic + agent-evaluated), MCP server with 8 tools (spec_list, spec_get, gate_run, gate_report, gate_finalize, gate_history, context_diagnose, estimate_tokens), run history with atomic persistence, required/advisory enforcement, session diagnostics, pruning engine, guard daemon, CLI (init, spec, gate, mcp serve), and Claude Code plugin with hooks.

---

## Executive Summary

Research across agent orchestration tools (agtx, Claude Code SDK), code review platforms (GitHub PR API, Gerrit), TUI frameworks (ratatui ecosystem), and multi-agent session patterns reveals clear guidance for Assay's v0.3.0 features.

**Key findings:**

1. **Git worktree lifecycle:** agtx provides the definitive reference implementation -- worktrees under `.agtx/worktrees/<task-slug>`, branch per task (`task/<slug>`), agent config directories auto-copied, init scripts, and `git worktree remove --force` for cleanup. The pattern is proven and Assay should adopt it with minimal deviation, storing worktrees under `.assay/worktrees/` instead.

2. **Headless agent launching:** Claude Code's `--print` mode (also called non-interactive mode) is the industry-standard for programmatic agent invocation. It supports `--output-format json` for structured output (returning `session_id`, `result`, metadata), `--continue`/`--resume <session_id>` for conversation continuity, and piped input. agtx's `generate_text()` abstraction shows how to wrap this per-agent. For Assay, the key insight is that `--print` mode is sufficient for independent gate evaluation -- no tmux orchestration needed.

3. **Session record tracking:** agtx uses SQLite with UUID-based task IDs, per-project databases keyed by path hash, and a global index DB. For Assay's lighter-weight needs, the existing JSON file approach (atomic writes, `.assay/results/`) is the right persistence layer. Adding a session record type that tracks worktree path, branch name, agent identity, and Claude Code session_id provides the lineage without introducing SQLite.

4. **Independent gate evaluation with diff context:** GitHub's unified diff format (via `git diff`) is the universal input for AI code review. The pattern is: assemble diff hunks, attach file paths and line ranges, and present as structured context alongside the gate criteria. No tool does this via MCP yet -- Assay's `gate_evaluate` would be genuinely novel.

5. **Minimal TUI gate results viewer:** ratatui's `Table` widget with `TableState` for scrolling, color-coded pass/fail rows, and a `Paragraph` detail pane is the proven pattern for read-only results viewers. The existing Assay TUI is a scaffold -- it needs a `Table` + detail split layout, 4-5 keybindings, and nothing more.

6. **Quick wins:** CLI correctness, MCP validation, types hygiene, error messages, and output truncation are table-stakes hardening. Every tool in the ecosystem that matured past v0.1 addressed these before adding features.

7. **Radical seeds:** Composable gate definitions (like agtx's plugin TOML), spec preconditions (like CI pipeline `needs:`), and gate history summaries (like SonarQube trend dashboards) are differentiators that set up v0.4+.

---

## 1. Git Worktree Lifecycle Management

### How agtx Handles Worktrees

agtx is the most complete reference implementation for agent-orchestrated worktree management in the Rust ecosystem.

**Creation (`create_worktree`):**
- Worktrees stored at `<project>/.agtx/worktrees/<task-slug>/`
- Branches named `task/<slug>`, based off detected main branch (`main`/`master`/current)
- Idempotent: returns existing worktree if valid (`.git` exists), cleans up partial state otherwise
- Uses `git worktree add <path> -b <branch> <base>` -- single atomic git command
- Deletes stale branches from previous failed attempts before creation

**Initialization (`initialize_worktree`):**
- Copies agent config directories (`.claude/`, `.gemini/`, `.codex/`, `.github/agents/`, `.config/opencode/`) from project root via recursive copy
- Copies plugin-specified extra directories
- Copies user-specified files from `copy_files` config (comma-separated)
- Runs `init_script` via `sh -c` in worktree directory
- Returns warnings (not errors) for any failures -- never blocks worktree creation
- Agent config constants defined as `AGENT_CONFIG_DIRS`

**Removal (`remove_worktree`):**
- `git worktree remove --force <path>` -- force flag handles uncommitted changes
- Falls back to `git worktree prune` if remove fails
- No branch cleanup (branches persist as references)

**Key Design Decisions:**
- `.agtx/` added to `.gitignore` -- worktrees are local-only state
- No symlink tricks -- real git worktrees with full checkout
- Task slug used as directory name (sanitized from title)
- Worktree path stored on `Task` model for later reference

### Assay Adaptation

Assay should store worktrees under `.assay/worktrees/<spec-slug>/` to maintain the existing `.assay/` namespace. Key differences from agtx:

| Concern | agtx | Assay v0.3.0 |
|---------|------|-------------|
| Storage | `.agtx/worktrees/<slug>` | `.assay/worktrees/<slug>` |
| Branch naming | `task/<slug>` | `assay/<spec-slug>` |
| Config copy | Agent dirs + user files | `.assay/` dir (specs, config) |
| Init script | User-configurable | Not needed for v0.3.0 |
| DB tracking | SQLite with worktree_path column | JSON session record |
| Cleanup | `git worktree remove --force` | Same, plus branch cleanup option |

**Expected User Behavior:**
1. `assay worktree create <spec-name>` -- creates worktree branched from current HEAD, copies `.assay/` config
2. Agent works in worktree, runs gates, produces changes
3. `assay worktree status` -- lists active worktrees with branch and diff summary
4. `assay worktree remove <spec-name>` -- cleans up worktree and optionally the branch
5. MCP tool `worktree_create` and `worktree_remove` for agent self-service

### Risk: Worktree Lifecycle is Fragile

Git worktrees have sharp edges: locked worktrees, stale references, orphaned branches, and the `.git` file (not directory) that worktrees use can confuse tools. agtx handles this with `--force` and `prune` fallbacks, which is the pragmatic approach. Assay should do the same and provide `assay worktree prune` for manual cleanup.

---

## 2. Claude Code Headless Launcher (`--print` Mode)

### How Claude Code's `--print` Mode Works

Claude Code's non-interactive mode is invoked with `-p` or `--print`:

```bash
# Basic usage
claude -p "Summarize this project"

# Structured JSON output
claude -p "List API endpoints" --output-format json

# Streaming JSON (NDJSON)
claude -p "Analyze performance" --output-format stream-json

# Session continuation
session_id=$(claude -p "Start review" --output-format json | jq -r '.session_id')
claude -p "Continue review" --resume "$session_id"

# Continue most recent session
claude -p "Follow up" --continue
```

**Output format (JSON):**
Returns structured data including `result` (text), `session_id`, and metadata. The `session_id` enables multi-turn conversations in headless mode.

**How agtx Uses It:**
agtx's `CodingAgent::generate_text()` dispatches per-agent:
- Claude: `claude --print <prompt>`
- Codex: `codex exec --full-auto <prompt>`
- Copilot: `copilot -p <prompt>`
- Gemini: `gemini -p <prompt>`

For interactive sessions, agtx uses `claude --dangerously-skip-permissions` inside tmux windows. The `--session <task_id>` flag persists conversation context across restarts.

### Assay's `gate_evaluate` Design

For independent gate evaluation, Assay needs to launch a headless Claude Code instance that:
1. Receives the gate criteria + diff context as input
2. Evaluates each criterion against the code changes
3. Returns structured pass/fail + evidence + reasoning

The `--print` mode with `--output-format json` is the right interface. The flow:

```
assay gate evaluate <spec-name>
  1. Compute git diff (working tree or branch comparison)
  2. Load spec criteria (agent-evaluated ones)
  3. Build prompt: criteria + diff context + evaluation rubric
  4. Invoke: claude -p "<prompt>" --output-format json
  5. Parse response, map to AgentEvaluation structs
  6. Submit via gate_report / gate_finalize flow
```

**No tmux needed.** Unlike agtx's interactive sessions, Assay's independent evaluation is a request-response pattern. `--print` mode handles this cleanly.

**Session continuation is optional but valuable.** If the evaluation needs follow-up ("explain why criterion X failed"), the session_id from the JSON output enables `--resume`.

### Multi-Agent Support

agtx's `AgentRegistry` pattern (trait-based dispatch with per-phase agent selection) is over-engineered for Assay's v0.3.0 needs. A simpler approach:

- Default to `claude -p` for independent evaluation
- Config key `evaluator_command` in `.assay/config.toml` for override
- Future: trait-based dispatch if multi-agent becomes a real use case

---

## 3. Session Record Tracking

### How Existing Tools Track Sessions

| Tool | Session Model | Storage | Key Fields |
|------|--------------|---------|------------|
| agtx | `Task` + `RunningAgent` | SQLite (per-project DB keyed by path hash) | id, title, status, agent, worktree_path, branch_name, session_name, pr_url, plugin, cycle |
| Claude Code | Session ID (UUID) | `~/.claude/projects/` transcript files | session_id, transcript_path, cwd, permission_mode |
| GitHub Actions | Run + Job + Step | API (server-side) | run_id, workflow_name, status, conclusion, started_at, completed_at |

### agtx Session Model (Deep Dive)

agtx tracks sessions through two complementary models:

**Task (persistent, SQLite):**
- UUID-based `id`, human `title`, optional `description`
- `status`: Backlog -> Planning -> Running -> Review -> Done
- `agent`: which agent is assigned
- `worktree_path`, `branch_name`: git context
- `session_name`: tmux session identifier
- `pr_number`, `pr_url`: PR lifecycle tracking
- `plugin`: which workflow plugin governs this task
- `cycle`: iteration count for cyclic workflows

**RunningAgent (runtime, SQLite):**
- `session_name`: tmux session (primary key)
- `task_id`, `project_id`: foreign keys
- `agent_name`: which agent is running
- `started_at`: when the session began
- `status`: Running / Waiting / Completed

**PhaseStatus (runtime-only, not persisted):**
- Working / Idle / Ready / Exited
- Detected by monitoring tmux pane output and artifact files

### Assay Adaptation: Session Records

Assay already has `AgentSession` (in-memory, for accumulating gate evaluations) and `GateRunRecord` (persisted, the result). What's missing is a **session record** that ties together:

1. The worktree used for this work
2. The Claude Code session_id (for resumption)
3. The spec being worked on
4. The gate run results produced
5. Timeline (created, last activity, completed)

**Proposed type:**
```
SessionRecord {
    session_id: String,          // Assay-generated, like run_id
    spec_name: String,
    worktree_path: Option<String>,
    branch_name: Option<String>,
    agent_session_id: Option<String>,  // Claude Code session_id for --resume
    gate_run_ids: Vec<String>,   // References to GateRunRecord files
    status: SessionStatus,       // Active / Completed / Abandoned
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

**Storage:** JSON files under `.assay/sessions/<session-id>.json`, same atomic write pattern as run history. No SQLite -- consistent with existing Assay architecture.

**Why not SQLite?** Assay is a single-project tool with low-volume session data (tens to hundreds of sessions, not thousands). JSON files are human-readable, git-diffable, and don't require a new dependency. agtx uses SQLite because it manages multiple projects from a global dashboard -- a different use case.

---

## 4. Independent Gate Evaluation (`gate_evaluate` with Diff Context)

### How Code Review Tools Assemble Diff Context

**Git unified diff format:**
The universal input for AI code review is `git diff` output -- unified diff with file paths, hunk headers (`@@ -start,count +start,count @@`), and context lines. Every tool that feeds code changes to an AI model uses this format or a close derivative.

**GitHub Pull Request Reviews:**
- `GET /repos/{owner}/{repo}/pulls/{number}/files` returns per-file patches
- Each file includes `filename`, `status` (added/modified/removed), `patch` (unified diff), `additions`, `deletions`
- Review comments reference specific diff positions via `position` (line within the diff) or `line` (file line number)
- The diff includes 3 lines of context by default

**Gerrit:**
- Provides per-file diffs with configurable context lines
- Comments anchor to file + line + side (left=old, right=new)
- Supports "related changes" to show dependency chains

**AI code review tools (CodeRabbit, PR-Agent, etc.):**
- Fetch the full PR diff
- Split into per-file chunks for token management
- Include file metadata (language, path in project)
- Provide the spec/guidelines as system context
- Ask for structured output (pass/fail per criterion, with evidence)

### Diff Context Assembly for Assay

For `gate_evaluate`, Assay needs to:

1. **Compute the diff:**
   - Default: `git diff HEAD` (uncommitted changes in working tree)
   - With worktree: `git diff main...<branch>` (all changes on the feature branch)
   - Configurable: `--base <ref>` flag for custom base

2. **Structure the context:**
   ```
   [Spec: <name>]
   [Description: <description>]

   [Criteria to evaluate:]
   1. <criterion-name>: <criterion-description> [enforcement: required/advisory]
   2. ...

   [Code changes (unified diff):]
   --- a/src/foo.rs
   +++ b/src/foo.rs
   @@ -10,5 +10,8 @@
   ...
   ```

3. **Handle large diffs:**
   - Truncate to fit context window (track token budget)
   - Prioritize files that match criterion patterns (if any)
   - Include file list even when diff is truncated

4. **Parse the response:**
   - Expect structured JSON: `{ criteria: [{ name, passed, evidence, reasoning, confidence }] }`
   - Use `--json-schema` flag with Claude Code `--print` mode for guaranteed structure
   - Map to `AgentEvaluation` structs and feed into existing `gate_report` flow

### MCP Tool Design: `gate_evaluate`

```
gate_evaluate {
    name: String,         // spec name
    base_ref: Option<String>,  // git ref for diff base (default: HEAD or main)
    include_evidence: bool,
    evaluator_role: Option<String>,  // "independent" (default), "self_eval"
}
```

This tool would:
1. Compute diff context
2. Launch headless agent
3. Collect evaluations
4. Create session, report evaluations, finalize
5. Return the GateRunRecord

The key differentiator: this is a **single-tool invocation** that handles the entire evaluate-report-finalize lifecycle, unlike the current 3-step flow (gate_run -> gate_report -> gate_finalize) which requires the agent to self-orchestrate.

---

## 5. Minimal TUI Gate Results Viewer

### ratatui Patterns for Results Viewers

ratatui provides everything needed for a minimal gate results viewer:

**Table widget:**
- `Table::new(rows, widths)` with `Row` and `Cell` types
- `TableState` for selection tracking and scrolling
- `row_highlight_style` for selected row
- Column widths via `Constraint::Length`, `Constraint::Min`, `Constraint::Percentage`
- `Scrollbar` widget for long result lists

**Layout:**
- `Layout::vertical([Constraint::Min(5), Constraint::Length(N)])` for table + detail split
- `Layout::horizontal([...])` for side-by-side panels

**Styling for pass/fail:**
- Green foreground for passed (`Style::default().fg(Color::Green)`)
- Red foreground + `Modifier::BOLD` for failed
- Yellow for skipped/advisory
- Alternating row backgrounds for readability

### Recommended TUI Architecture

The existing Assay TUI (`crates/assay-tui/src/main.rs`) is a minimal scaffold. For a gate results viewer:

**Layout:**
```
+--------------------------------------------------+
| Assay Gate Results: <spec-name>         [q] quit  |
+--------------------------------------------------+
| # | Criterion      | Status | Enforcement | Time |
|---|----------------|--------|-------------|------|
| 1 | cargo-test     | PASS   | required    | 1.5s |
| 2 | clippy-lint    | FAIL   | advisory    | 0.8s |
| 3 | code-review    | PASS   | required    | agent|
| 4 | readme-exists  | PASS   | required    | 0ms  |
+--------------------------------------------------+
| Detail: clippy-lint                               |
| Status: FAIL (advisory)                           |
| stderr: warning: unused variable `x`              |
| Exit code: 1 | Duration: 800ms | Truncated: yes  |
+--------------------------------------------------+
| Summary: 3/4 passed | Required: 3/3 | Advisory: 0/1|
+--------------------------------------------------+
```

**Keybindings (minimal):**
- `j`/`k` or arrows: navigate rows
- `Enter`: toggle detail pane expansion
- `q`/`Esc`: quit
- `r`: re-run gates (optional, v0.3.0 stretch)

**Data source:**
- Load from `GateRunRecord` (most recent, or specified by run_id)
- Or run gates live and display results as they arrive

**Implementation approach:**
- Single `App` struct with `TableState` and selected `GateRunRecord`
- `Widget` trait implementation for the main view
- ~200-300 lines total for a functional viewer

### What agtx Does Differently

agtx's TUI is a full kanban board with 5 columns, task creation, agent launching, tmux integration, sidebar panels, file search, and plugin selection. This is massively over-scoped for Assay v0.3.0. The key lesson is agtx's **column-based navigation pattern** (h/l for columns, j/k for items) which works well for status-oriented data.

For Assay, the single-table-with-detail-pane pattern is more appropriate. It maps directly to gate results (one row per criterion) without the complexity of multi-column state management.

---

## 6. Quick Wins (Hardening)

### CLI Correctness
- **Table stakes.** Every mature CLI tool validates inputs, provides helpful error messages, and exits with correct codes.
- Key items: argument validation before execution, consistent exit codes (0 = all gates pass, 1 = gate failure, 2 = infrastructure error), `--help` text accuracy.

### MCP Validation
- **Table stakes.** MCP tool parameters must be validated before execution -- reject malformed inputs with clear error messages rather than panicking.
- Key items: parameter bounds checking, spec name existence validation before gate operations, session_id format validation.

### Types Hygiene
- **Table stakes.** `deny_unknown_fields` on all serialized types (already done), `#[non_exhaustive]` on public enums for forward compatibility, derive consistency across the types crate.
- Key items: audit all public types in `assay-types` for missing derives, ensure `schemars::JsonSchema` coverage is complete.

### Error Messages
- **Table stakes.** Actionable errors that tell the user what went wrong AND what to do about it.
- Pattern: "Failed to load spec 'foo': file not found at .assay/specs/foo.toml. Run `assay spec list` to see available specs."

### Output Truncation
- **Table stakes.** Gate command output is already truncated at 64KB (`MAX_OUTPUT_BYTES`). Ensure MCP responses are also bounded, and that truncation is clearly signaled to the agent.

---

## 7. Radical Seeds

### 7a. Composable Gate Definitions

**Inspiration:** agtx's plugin TOML framework, where a single TOML file defines phases, skills, prompts, and artifacts for an entire development methodology.

**For Assay:** Gate definitions could be composed from reusable building blocks:

```toml
# .assay/gates/rust-quality.toml (reusable gate template)
[gate]
name = "rust-quality"
criteria = [
    { name = "format", cmd = "cargo fmt --check" },
    { name = "lint", cmd = "cargo clippy -- -D warnings" },
    { name = "test", cmd = "cargo test" },
    { name = "deny", cmd = "cargo deny check" },
]

# .assay/specs/auth-flow/gates.toml
[gate]
inherit = ["rust-quality"]  # Pull in all criteria from the template
criteria = [
    { name = "auth-review", description = "Security review of auth flow", kind = "agent" },
]
```

**Categorization:** Differentiator. No spec-driven agent tool offers composable gate templates yet. This would reduce boilerplate significantly for monorepos with many specs sharing common quality checks.

### 7b. Spec Preconditions

**Inspiration:** CI pipeline `needs:` / `depends_on` declarations (GitHub Actions, GitLab CI).

**For Assay:** Specs could declare dependencies on other specs' gates passing:

```toml
[spec]
name = "deploy-auth"
preconditions = ["auth-flow", "db-migration"]  # These specs' gates must pass first
```

**Categorization:** Differentiator. This enables workflow DAGs without a separate orchestration layer. However, it adds complexity to the evaluation model (topological sort, cycle detection, partial evaluation).

### 7c. Gate History Summary

**Inspiration:** SonarQube's quality gate trend dashboard, showing pass/fail rates over time, flaky gate detection, and performance trends.

**For Assay:** An MCP tool `gate_history_summary` that returns:
```json
{
    "spec_name": "auth-flow",
    "total_runs": 15,
    "pass_rate": 0.73,
    "criteria_stats": [
        { "name": "cargo-test", "pass_rate": 1.0, "avg_duration_ms": 1500 },
        { "name": "code-review", "pass_rate": 0.6, "avg_duration_ms": 0, "flaky": true }
    ],
    "trend": "improving",
    "last_5_results": ["pass", "pass", "fail", "pass", "pass"]
}
```

**Categorization:** Differentiator. Agents could use this to prioritize which criteria to focus on, identify flaky gates, and track quality trends. The data is already available in `.assay/results/` -- this is pure aggregation logic.

---

## Feature Categorization

### Table Stakes (Must have for v0.3.0 credibility)

| Feature | Rationale |
|---------|-----------|
| CLI correctness (exit codes, validation) | Every CLI tool does this |
| MCP parameter validation | Prevents agent confusion from silent failures |
| Types hygiene (`non_exhaustive`, derive audit) | Forward compatibility for plugins |
| Actionable error messages | Users can't self-help with opaque errors |
| Output truncation consistency | MCP responses must be bounded |
| Session record tracking (basic) | Agents need to track what they're working on |

### Differentiators (Unique to Assay, competitive advantage)

| Feature | Rationale |
|---------|-----------|
| `gate_evaluate` with diff context | No MCP tool does single-invocation AI code review against spec criteria |
| Composable gate definitions (radical seed) | Reduces boilerplate, no competitor offers this |
| Gate history summary (radical seed) | Agents can use trend data for prioritization |
| Spec preconditions (radical seed) | Workflow DAGs without external orchestration |
| Minimal TUI gate viewer | Visual feedback loop for spec-driven development |

### Anti-features (Avoid for v0.3.0)

| Feature | Rationale |
|---------|-----------|
| Full kanban TUI (agtx-style) | Over-scoped; Assay is a quality gate tool, not a project manager |
| SQLite for session storage | JSON files are sufficient for single-project scope; SQLite adds dependency and migration burden |
| tmux-based agent orchestration | `--print` mode is simpler and more reliable for evaluation; tmux is for interactive sessions |
| Multi-agent registry with per-phase dispatch | YAGNI for v0.3.0; single evaluator command is sufficient |
| Worktree init scripts | Adds configuration surface without clear value for quality gate workflows |
| Agent config directory auto-copy (agtx-style) | Assay's `.assay/` dir is the only config needed in worktrees |
| Plugin framework (agtx-style TOML plugins) | Premature abstraction; composable gate definitions are the right first step |
| PR lifecycle management | Out of scope; Assay evaluates quality, not manages PRs |
| Cyclic workflow phases | Assay's workflow is linear: spec -> gate -> pass/fail, not a kanban loop |

---

## Implementation Priority Recommendations

### Phase 1: Foundation (Quick Wins + Session Records)
1. CLI correctness and error message improvements
2. MCP parameter validation hardening
3. Types hygiene audit
4. Output truncation for MCP responses
5. `SessionRecord` type in `assay-types`, persistence in `assay-core`

### Phase 2: Core Features (Worktrees + Headless Evaluation)
1. `assay worktree create/remove/status` CLI commands
2. MCP tools: `worktree_create`, `worktree_remove`
3. Diff context assembly (`git diff` parsing and structuring)
4. `gate_evaluate` MCP tool (headless Claude Code launcher + diff context + auto-finalize)

### Phase 3: Visualization (TUI Viewer)
1. Table-based gate results viewer with pass/fail coloring
2. Detail pane for selected criterion (stdout/stderr/evidence)
3. Summary footer with enforcement breakdown
4. Load from run history or live evaluation

### Phase 4: Seeds (Radical Features for v0.4.0 Preparation)
1. Composable gate definitions (`inherit` in gates.toml)
2. Gate history summary aggregation
3. Spec preconditions (dependency declaration, no execution yet)

---

## Appendix A: agtx Source Code Analysis

### Key Files Examined
- `src/git/worktree.rs` -- Full worktree lifecycle (create, initialize, remove, exists check)
- `src/agent/mod.rs` -- Agent registry, `build_interactive_command()`, `build_spawn_args()`, `--print` mode dispatch
- `src/agent/operations.rs` -- `AgentOperations` trait, `generate_text()` headless invocation, `AgentRegistry` pattern
- `src/db/models.rs` -- `Task`, `Project`, `RunningAgent`, `PhaseStatus` data models
- `src/db/schema.rs` -- SQLite schema (tasks, projects, running_agents tables), migration pattern
- `src/tmux/operations.rs` -- `TmuxOperations` trait, window/session management, pane capture

### Architecture Pattern
```
agtx TUI (ratatui) -> Board State -> Database (SQLite)
                                  -> Git Operations (worktrees, branches)
                                  -> Tmux Operations (sessions, windows)
                                  -> Agent Operations (launch, generate_text)
```

Trait-based abstractions (`AgentOperations`, `TmuxOperations`, `GitOperations`) with `mockall` for testing. Production implementations dispatch to CLI commands (`git`, `tmux`, `claude`). This is a clean architecture but heavier than Assay needs.

### Key Constants
- Agent server name: `agtx` (tmux `-L agtx`)
- Agent config dirs: `.claude/`, `.gemini/`, `.codex/`, `.github/agents/`, `.config/opencode/`
- Branch naming: `task/<slug>`
- Database path: `~/Library/Application Support/agtx/projects/<path-hash>.db`

## Appendix B: Claude Code `--print` Mode Reference

### Invocation Patterns
```bash
# Basic (text output)
claude -p "<prompt>"

# JSON output with session_id
claude -p "<prompt>" --output-format json

# Streaming NDJSON
claude -p "<prompt>" --output-format stream-json

# Session continuation
claude -p "<follow-up>" --resume "<session_id>"
claude -p "<follow-up>" --continue  # most recent session

# With JSON schema for structured output
claude -p "<prompt>" --output-format json --json-schema '<schema>'
```

### Output Structure (JSON mode)
```json
{
    "result": "...",
    "session_id": "abc123",
    // additional metadata
}
```

### Relevance to Assay
- `--print` + `--output-format json` is the interface for `gate_evaluate`
- `--json-schema` can enforce structured evaluation responses
- `session_id` enables multi-turn evaluation if initial assessment is ambiguous
- `--resume` enables "explain this failure" follow-ups
- No tmux, no daemon, no persistent process -- clean request-response

## Appendix C: ratatui Patterns for Gate Results Viewer

### Recommended Widget Stack
1. **Table** -- Main results grid (criterion name, status, enforcement, duration)
2. **Paragraph** -- Detail pane for selected criterion (stdout, stderr, evidence, reasoning)
3. **Block** -- Borders and titles for visual structure
4. **Scrollbar** -- For long result lists
5. **Layout** -- Vertical split: header + table + detail + footer

### State Management
- `TableState` for selection and scroll position
- Single `App` struct with `should_exit`, `table_state`, `record: GateRunRecord`
- Event loop: `terminal.draw()` + `event::read()` + key dispatch

### Color Scheme (Pass/Fail)
- Pass: green foreground (`Color::Green`)
- Fail + Required: red foreground + bold (`Color::Red` + `Modifier::BOLD`)
- Fail + Advisory: yellow foreground (`Color::Yellow`)
- Skipped: dark gray (`Color::DarkGray`)
- Selected row: reversed or background highlight
- Alternating row backgrounds for readability
