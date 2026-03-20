---
name: next-chunk
description: >
  Show the active chunk's full criteria list so you know exactly what to implement.
  Use before starting implementation on a new chunk, or when you need to know
  what success looks like for the current unit of work.
---

# Next Chunk

Load the active chunk and display its complete criteria so implementation can begin.

## Steps

1. **Call `cycle_status`:**
   - If the response is `{"active":false}`, print:
     "No active milestone — run `/assay:plan` to create one."
     Then stop.

2. **Call `chunk_status` with the `active_chunk_slug`:**
   - If `has_history` is `false`, note: "No gate runs yet for this chunk."
   - If `has_history` is `true`, show the latest pass/fail/required_failed counts.

3. **Call `spec_get` with the `active_chunk_slug`:**
   - Load all criteria for the active chunk

4. **Display the chunk detail:**
   - Chunk slug and name
   - Gate status (from step 2)
   - Full criteria list (from step 3), grouped by executable vs descriptive

## Output Format

Show the chunk summary first, then the full criteria list. Example:

```
Active chunk: login
Gate status:  No gate runs yet

Criteria:
  [executable] build passes — cargo build --release
  [executable] tests pass  — cargo test auth::login
  [descriptive] login endpoint returns 401 on bad credentials
```

This gives you the full implementation target. After implementing, run `/assay:gate-check login` to verify.
