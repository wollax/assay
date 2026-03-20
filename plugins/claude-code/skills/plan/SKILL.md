---
name: plan
description: >
  Guide the user through creating a new Assay milestone with chunks and specs.
  Use when the user wants to start a new development milestone, define a feature,
  or set up a milestone-driven development cycle. Interviews the user before
  making any MCP calls — never creates immediately on invocation.
---

# Plan

Create a new milestone with chunks and specs through a guided interview.

## Steps

1. **Interview the user (do this first — no MCP calls yet):**
   - Ask for the milestone goal / name (you will derive a slug: lowercase, hyphen-separated, e.g. `auth-refresh`)
   - Ask how many chunks the milestone has (1–7 recommended)
   - For each chunk, ask:
     - A name (e.g. "Core API changes")
     - A slug (e.g. `core-api`, derived from the name if not provided)
     - Two to four success criteria as plain descriptions (e.g. "POST /token returns 200 with valid body")
   - Confirm the full plan with the user before proceeding

2. **Create the milestone:**
   - Call `milestone_create` with:
     ```json
     {
       "slug": "<milestone-slug>",
       "name": "<Milestone Name>",
       "chunks": [
         { "slug": "<chunk-slug>", "name": "<Chunk Name>" }
       ]
     }
     ```

3. **Create specs (one per chunk):**
   - For each chunk, call `spec_create` with:
     ```json
     {
       "slug": "<chunk-slug>",
       "name": "<Chunk Name>",
       "milestone_slug": "<milestone-slug>",
       "criteria": ["<criterion 1>", "<criterion 2>"]
     }
     ```
   - Criteria are plain text descriptions — they do **not** need to be executable commands yet

4. **Confirm and warn:**
   - Show a summary: milestone slug, chunk count, spec files created
   - Remind the user: **Generated gate files have no `cmd` field** — they contain descriptive criteria only. To make gates runnable, edit each `specs/<chunk-slug>/gates.toml` and add a `cmd` field to each gate entry.
   - Suggest next steps: use `/assay:status` to see cycle progress, or `assay milestone advance` to evaluate gates when ready

## Output Format

Keep the interview conversational. After creation, show a concise confirmation:

```
✓ Milestone created: my-feature (3 chunks)
  Specs created: chunk-1, chunk-2, chunk-3

⚠ Gates have no cmd — edit specs/<slug>/gates.toml to add runnable commands.

Next: /assay:status to check cycle progress
```
