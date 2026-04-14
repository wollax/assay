# Assay Workflow

This project uses Assay for milestone-driven spec development. Use the skills and commands below to work through the development cycle.

## Skills

| Skill | Description |
| --- | --- |
| `/assay:explore` | Load project context for brainstorming and requirements discovery |
| `/assay:plan` | Interview user → create milestone + specs (supports `quick` for flat specs) |
| `/assay:focus` | Show active work: chunk criteria, gate status, progress |
| `/assay:check [name]` | Run quality gates with smart routing, suggest next step |
| `/assay:ship` | Verify gates pass → create PR with evidence |
| `/assay:spec-show [name]` | Display a spec's criteria and details |

## CLI Commands

| Command | Description |
| --- | --- |
| `assay plan` | Interactive milestone wizard (CLI) |
| `assay milestone list` | List all milestones |
| `assay milestone status` | Show in-progress milestone progress |
| `assay milestone advance` | Evaluate gates and mark active chunk complete |
| `assay pr create <slug>` | Gate-gated PR creation via `gh` |

## MCP Tools

| Tool | Description |
| --- | --- |
| `milestone_list` | List all milestones |
| `milestone_get` | Get a milestone by slug |
| `milestone_create` | Create a milestone with chunks |
| `spec_list` | List available specs |
| `spec_get` | Get a spec's full definition |
| `spec_create` | Create a spec with criteria |
| `cycle_status` | Get active milestone and chunk state |
| `cycle_advance` | Advance the cycle to the next chunk (confirm gates first with `chunk_status`) |
| `chunk_status` | Get gate pass/fail summary for a chunk |
| `pr_create` | Create a gate-gated PR |
| `gate_run` | Run quality gates for a spec |
