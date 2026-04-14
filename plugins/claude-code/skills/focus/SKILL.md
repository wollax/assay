---
name: focus
description: >
  Show what you're currently working on — active chunk, criteria, and gate status.
  Use when the user asks "what am I working on?", "what's next?", or wants to see current progress.
  Replaces /assay:status and /assay:next-chunk.
---

# Focus

Show active work context: milestone, chunk, criteria, and gate status.

## Steps

1. **Call `cycle_status`** to get the active milestone and chunk

2. **Handle no active milestone:**
   - Tell the user: *"No active milestone. Start with `/assay:plan` or `/assay:explore`."*

3. **Call `chunk_status`** with the active chunk slug to get gate pass/fail per criterion

4. **Call `spec_get`** with the active chunk slug to load full criteria

5. **For quick milestones** (single chunk matching milestone slug):
   - Hide milestone/chunk terminology
   - Show as: "Working on: [spec name]" with criteria and gate status

6. **Display:**
   - Spec name and status (draft/ready/approved/verified)
   - Criteria list with pass/fail indicators
   - Progress: chunks completed / total
   - Suggested next action based on state

## Output Format

Compact view — criteria table with pass/fail status. One-line progress summary. Suggest the logical next step.
