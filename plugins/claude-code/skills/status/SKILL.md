---
name: status
description: >
  Show the current development cycle status — active milestone, phase, chunk, and progress.
  Use when the user asks "what am I working on?", "where are we?", or wants a progress overview.
---

# Status

Show active milestone and chunk state from the development cycle.

## Steps

1. **Call `cycle_status`** (no parameters required)

2. **Handle the response:**
   - If `{ "active": false }`: tell the user no milestone is currently in progress.
     Suggest: *"Start a new milestone with `/assay:plan`, or list existing ones with `assay milestone list`."*
   - If an active milestone exists, display:
     - Milestone name and slug
     - Current phase
     - Active chunk slug (the chunk currently being worked on)
     - Progress: `completed_count / total_count` chunks done

## Output Format

Keep output concise — one short block is enough. Example:

```
Milestone: my-feature (my-feature)
Phase:     Execute
Chunk:     auth-layer  (1 of 3 complete)
```
