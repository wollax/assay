---
name: cycle-status
description: >
  Show an overview of the active milestone and chunk progress.
  Use when the user wants to know where they are in the development
  cycle, how many chunks are complete, and whether the active chunk
  has passing gates.
---

# Cycle Status

Display a concise overview of the active milestone, chunk completion, and latest gate results.

## Steps

1. **Call `cycle_status`:**
   - If the response is `{"active":false}`, print:
     "No active milestone — run `/assay:plan` to create one."
     Then stop.

2. **Call `chunk_status` for the active chunk:**
   - Use the `active_chunk_slug` field from the `cycle_status` response
   - If `has_history` is `false`, note: "No gate runs yet — implement the chunk then run `/assay:gate-check <chunk-slug>`"

3. **Display the overview:**
   - Milestone name and phase
   - Chunk progress: X of N chunks complete
   - Active chunk slug
   - If gate history exists: latest pass / fail / required_failed counts

## Output Format

Use a concise table or short bulleted list. Example:

```
Milestone:     auth-layer (InProgress)
Progress:      2 / 5 chunks complete
Active chunk:  login
Gate status:   3 passed, 1 failed (1 required)
```

Keep it to 6 lines or fewer. For chunk detail and full criteria, use `/assay:next-chunk`.
