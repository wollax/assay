---
name: next-chunk
description: >
  Show the active chunk's spec criteria and gate status. Use before starting work on a chunk,
  to understand what needs to pass, or to check current gate results.
---

# Next Chunk

Load context for the active chunk: cycle state, gate status, and spec criteria.

## Steps

1. **Call `cycle_status`** (no parameters):
   - If `{ "active": false }`: stop and tell the user no milestone is active. Suggest `/assay:plan` to create one.
   - If `active_chunk_slug` is `null`: all chunks are complete and the milestone is in the Verify phase. Tell the user: *"All chunks are complete. Create a PR with `assay pr create <milestone-slug>` or use the `pr_create` tool."*
   - Otherwise proceed with `active_chunk_slug`

2. **Call `chunk_status`** with `{ "chunk_slug": "<active_chunk_slug>" }`:
   - Display a gate summary: `passed`, `failed`, `required_failed` counts
   - If `has_history: false`: note that no gate runs exist yet for this chunk

3. **Call `spec_get`** with `{ "name": "<active_chunk_slug>" }`:
   - Display the full criteria list for the chunk: criterion name, description, whether executable, and `cmd` if present

4. **Summarise:** State the active chunk name, gate pass/fail status, and what criteria must pass before calling `cycle_advance`.

## Output Format

Show gate status first (pass/fail counts), then criteria list. Keep it scannable — one line per criterion is enough for passing criteria. Flag failing or unrun criteria clearly so the user knows what to focus on.
