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
| `/assay:spec-show [name]` | Display a spec's criteria and details |
| `/assay:gate-check [name]` | Run quality gates and report results |

## MCP Tools

The Assay MCP server provides these tools directly:

| Tool | Description |
| --- | --- |
| **Specs** | |
| `spec_list` | Discover available specs in the project |
| `spec_get` | Get a spec's full definition with all criteria |
| `spec_validate` | Statically validate a spec without running it |
| **Gates** | |
| `gate_run` | Run quality gate criteria (auto-creates sessions for agent criteria) |
| `gate_evaluate` | Evaluate all criteria via headless Claude Code subprocess |
| `gate_report` | Submit agent evaluation for a criterion in an active session |
| `gate_finalize` | Finalize a session, persisting evaluations as a GateRunRecord |
| `gate_history` | Query past gate run results and track quality trends |
| **Context** | |
| `context_diagnose` | Diagnose token usage and bloat in a Claude Code session |
| `estimate_tokens` | Estimate current token usage and context window health |
| **Worktrees** | |
| `worktree_create` | Create an isolated git worktree for a spec |
| `worktree_list` | List all active assay-managed worktrees |
| `worktree_status` | Check worktree status (branch, dirty, ahead/behind) |
| `worktree_cleanup` | Remove a worktree and its branch |
| **Merge** | |
| `merge_check` | Check for merge conflicts between two refs (read-only) |
| **Sessions** | |
| `session_create` | Create a new work session for a spec |
| `session_get` | Retrieve full session details by ID |
| `session_update` | Transition session phase and link gate runs |
| `session_list` | List sessions with optional filters |
| **Orchestration** | |
| `run_manifest` | Run a manifest through the end-to-end pipeline |
| `orchestrate_run` | Run a multi-session manifest through the orchestrator |
| `orchestrate_status` | Read persisted orchestrator state for a run ID |
