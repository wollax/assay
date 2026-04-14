---
name: plan
description: >
  Guide the user through creating a new milestone with chunks and specs.
  Use when the user wants to start planning a new feature, task, or project milestone.
  Collects goal, chunk names, and acceptance criteria conversationally before calling any MCP tools.
---

# Plan

Collect all inputs from the user before creating anything, then create the milestone and specs.

## Steps

1. **Check for quick mode:**
   - If `$ARGUMENTS` contains "quick" or the user asks for a simple/flat spec:
     - Ask: *"What is the name of this task?"*
     - Ask: *"What are the acceptance criteria? List one per line."*
     - Call `milestone_create` with quick mode parameters (single chunk matching milestone slug)
     - Skip the full milestone interview below
     - Tell the user: *"Quick spec created. Run `/assay:focus` to see criteria."*
     - Stop here — do not continue to the multi-chunk flow

2. **Interview the user — do not call any tools yet:**
   - Ask: *"What is the goal of this milestone? Describe what you want to build or achieve."*
   - Ask: *"How many chunks do you want to break this into? (Suggest 2–5 for most milestones)"*
   - For each chunk, ask: *"What is the name of chunk N?"* — then derive the slug as lowercase with spaces replaced by hyphens (e.g. "Auth Layer" → `auth-layer`)
   - Ask: *"What are the acceptance criteria for [chunk name]? List one per line — each criterion should be verifiable."*
   - Repeat criteria collection for each chunk
   - Show a summary of what will be created and ask the user to confirm before proceeding

3. **Create the milestone:**
   - Call `milestone_create` with `{ slug, name, description, chunks: [{ slug, name, criteria: [] }, ...] }`
   - Pass `criteria: []` for each chunk here — specs are created per-chunk in Step 3

4. **Create a spec for each chunk:**
   - For each chunk, call `spec_create` with `{ slug: <chunk-slug>, name: <chunk-name>, milestone_slug: <milestone-slug>, criteria: ["<criterion 1>", "<criterion 2>", ...] }`
   - Pass each criterion exactly as the user entered it — one plain string per criterion
   - If `spec_create` returns `isError: true` (e.g. duplicate slug): tell the user which chunk failed and suggest a different slug. Do NOT silently continue to the next chunk.

5. **Confirm results:**
   - Report the created milestone slug and name
   - List each chunk with its spec path (typically `specs/<chunk-slug>.toml`)
   - Tell the user: *"Run `/assay:next-chunk` to see the first chunk's criteria and start working."*

## Output Format

During the interview, ask one question at a time. Keep the conversation focused. After creation, show a compact summary table of milestone slug, chunks, and spec locations.
