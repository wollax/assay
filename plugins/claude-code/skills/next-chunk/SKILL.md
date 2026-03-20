---
name: next-chunk
description: >
  Load the active chunk's spec and gate status for implementation context.
  Use when the user wants to know what to implement next, what the current
  criteria are, or which gates are passing/failing for the active chunk.
---

# Next Chunk

Load the active chunk's criteria and current gate status.

## Steps

1. **Find the active chunk:**
   - Call the `cycle_status` MCP tool (no parameters)
   - Extract `active_chunk_slug` from the response

2. **Handle no active chunk:**
   - If `active == false` or `active_chunk_slug` is null or missing, report:
     > No active chunk — all chunks complete (or no active milestone).
     > Use `assay milestone advance` to evaluate gates and complete the milestone, or `assay pr create <milestone-slug>` to create a PR.

3. **Fetch gate pass/fail summary:**
   - Call `chunk_status` with `{ "chunk_slug": "<active_chunk_slug>" }`
   - This returns pass/fail status for each gate in the chunk

4. **Fetch full spec criteria:**
   - Call `spec_get` with `{ "name": "<active_chunk_slug>" }`
   - This returns the full spec definition including all criteria descriptions

5. **Present the chunk context:**
   - Show:
     - **Chunk:** `active_chunk_slug`
     - **Criteria list:** for each criterion, show:
       - Name
       - Description
       - Status: ✓ pass or ✗ fail (from `chunk_status` results)
   - Suggest next action:
     - If all pass: "All gates passing — run `assay milestone advance` to mark this chunk complete"
     - If any fail: "Fix failing criteria, then run `/assay:gate-check <chunk-slug>` to re-verify"

## Output Format

```
Active chunk: chunk-2

Criteria:
  ✓ endpoint-returns-200   — POST /token returns 200 with valid body
  ✗ token-expiry-respected — Token expires after configured TTL
  ✓ error-on-bad-password  — Returns 401 for wrong credentials

1/3 passing — fix failing criteria, then run /assay:gate-check chunk-2
```
