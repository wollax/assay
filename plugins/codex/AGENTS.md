# Assay — Codex Agent Instructions

This project uses Assay for spec-driven development with quality gates. Each feature is defined as a spec with verifiable criteria; gates run commands to confirm the criteria are met. During an active milestone, work advances one chunk at a time through a structured cycle.

## Skills

| Command | Description |
| --- | --- |
| `/assay:gate-check [spec]` | Run quality gates and report pass/fail results |
| `/assay:spec-show [spec]` | Display a spec's criteria before implementing |
| `/assay:cycle-status` | Overview of the active milestone and chunk progress |
| `/assay:next-chunk` | Detail view of the active chunk with full criteria list |
| `/assay:plan` | Interview-guided creation of a new milestone and specs |

## MCP Tools

| Tool | Description |
| --- | --- |
| `spec_list` | List all specs in the project |
| `spec_get` | Get a spec's full definition and criteria |
| `gate_run` | Run quality gates for a spec |
| `cycle_status` | Get active milestone progress (`{"active":false}` if none) |
| `cycle_advance` | Advance the active chunk and move to the next |
| `chunk_status` | Get gate results for a specific chunk |
| `milestone_list` | List all milestones |
| `milestone_create` | Create a new milestone with chunks |
| `spec_create` | Create a spec for a chunk with criteria |
| `pr_create` | Open a pull request for the completed milestone |

## Workflow

1. **Plan:** Run `/assay:plan` to define a milestone and its chunks
2. **Read:** Run `/assay:next-chunk` to see the active chunk and its criteria
3. **Implement:** Write code that satisfies each criterion
4. **Gate-check:** Run `/assay:gate-check <chunk-slug>` — fix any failures
5. **Advance:** When all gates pass, call `cycle_advance` to mark the chunk complete; repeat from step 2
6. **PR:** When all chunks are done, commit and open a PR
