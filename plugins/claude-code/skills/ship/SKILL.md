---
name: ship
description: >
  Verify gates pass and create a PR with gate evidence.
  Use when the user wants to ship their work, create a PR, or merge changes.
---

# Ship

Verify gates and create a gate-gated PR with evidence.

## Steps

1. **Verify gates pass:**
   - Call `gate_run` for the active spec (or `$ARGUMENTS` if provided)
   - If any required criteria fail: report failures and stop — do not create PR

2. **Check milestone state:**
   - Call `cycle_status` to confirm current phase
   - If all chunks are complete: proceed to PR
   - If more chunks remain: ask the user if they want to ship the current chunk or wait

3. **Create PR:**
   - Call `pr_create` with the milestone slug
   - The tool automatically includes gate evidence in the PR body

4. **Report:**
   - Show the PR URL
   - Show gate summary (passed/failed counts)
   - If more chunks remain: suggest advancing to the next chunk

## Output Format

Gate verification first (one line), then PR creation result with URL. Keep it action-oriented.
