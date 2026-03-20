---
name: status
description: >
  Show current milestone cycle progress: active milestone, phase, active chunk,
  and completion count. Use when the user wants to know where they are in the
  development cycle or what to work on next.
---

# Status

Show current milestone cycle progress.

## Steps

1. **Fetch cycle state:**
   - Call the `cycle_status` MCP tool (no parameters)

2. **Handle inactive state:**
   - If the response contains `"active": false`, report:
     > No active milestone. Run `/assay:plan` to create one, or `assay milestone list` to see existing milestones.

3. **Display active cycle:**
   - Show the following fields from the response:
     - **Milestone:** `milestone_slug` — `milestone_name`
     - **Phase:** `phase` (e.g. `InProgress`, `Verify`)
     - **Active chunk:** `active_chunk_slug` (if null or missing, show "all chunks complete")
     - **Progress:** visual bar using `[x]` for each completed chunk and `[ ]` for each remaining, e.g. `[x][x][ ]` — derived from `completed_count` and `total_count`
   - Suggest next action:
     - If `active_chunk_slug` is set: "Use `/assay:next-chunk` to load the active chunk context"
     - If all chunks complete: "Run `assay milestone advance` to evaluate gates and complete the milestone"

## Output Format

Keep output concise:

```
Milestone: my-feature — My Feature
Phase:     InProgress
Chunk:     chunk-2  (2 of 3)
Progress:  [x][ ][ ]

Next: /assay:next-chunk to load chunk context
```
