# Assay Workflow

This project uses Assay for spec-driven development with quality gates.

## Workflow

Plan a milestone with `/assay:plan` (guides you through goal, chunks, and criteria). Work through chunks using `/assay:next-chunk` to see the active chunk's criteria and gate status. Run `/assay:gate-check` after implementing to verify criteria. When all chunk gates pass (confirm with `/assay:next-chunk` or the `chunk_status` MCP tool), call the `cycle_advance` MCP tool to move to the next chunk. When all chunks are complete, open a PR with `assay pr create <milestone-slug>`.

## Skills

| Skill | Description |
| --- | --- |
| `/assay:spec-show [name]` | Display a spec's criteria and details |
| `/assay:gate-check [name]` | Run quality gates and report results |
| `/assay:plan` | Interview-first milestone creation (chunks + criteria) |
| `/assay:status` | Show active milestone, phase, chunk, and progress |
| `/assay:next-chunk` | Show active chunk's criteria and gate status |

## MCP Tools

| Tool | Description |
| --- | --- |
| `spec_list` | List all specs in the project |
| `spec_get` | Get a spec's full definition with criteria |
| `gate_run` | Run quality gates for a spec |
| `milestone_list` | List all milestones |
| `milestone_get` | Get milestone details |
| `milestone_create` | Create a new milestone with chunks |
| `spec_create` | Create a spec with criteria for a chunk |
| `cycle_status` | Get active milestone, phase, and chunk |
| `cycle_advance` | Advance to the next chunk when gates pass |
| `chunk_status` | Get gate pass/fail summary for a chunk |
| `pr_create` | Create a PR for a completed milestone |
