---
name: plan
description: >
  Interview-guided creation of a new milestone with chunks and specs.
  Use when starting a new feature or project milestone.
  Collects all inputs before calling any MCP tools.
---

# Plan

Create a new milestone and its specs through a structured interview. **All user inputs are collected before any MCP tool is called.**

## Steps

### Step 1 — Gather milestone goal, name, and slug

Ask the user:
- What is the **goal** of this milestone? (one sentence)
- What is the **milestone name**? (human-readable, e.g. "Auth Layer")
- Propose a **slug** derived from the name (e.g. `auth-layer`); confirm or let the user override

Do not call any MCP tools yet.

### Step 2 — Gather chunk list

Ask the user:
- How many chunks make up this milestone? (1–7 recommended)
- For each chunk: what is its **name** and **slug**?

Slugs should be short and lowercase (e.g. `login`, `signup`, `session-refresh`). Propose slugs from the names and let the user confirm.

Do not call any MCP tools yet.

### Step 3 — Gather criteria per chunk

For each chunk in order, ask:
- What are the **success criteria** for `<chunk-slug>`? (1–5 text descriptions)

Criteria are plain text descriptions. They do **not** include shell commands — those are added manually after creation (see Output Format note below).

Do not call any MCP tools yet.

### Step 4 — Check for slug collision

Now call `milestone_list` to retrieve existing milestones.
If the proposed milestone slug already exists, warn the user and ask them to choose a different slug before proceeding.

### Step 5 — Create the milestone

Call `milestone_create` with:
- `slug` — confirmed milestone slug
- `name` — milestone name
- `description` — the goal from Step 1
- `chunks` — array of `{ slug, name }` for every chunk (criteria are provided separately via `spec_create` in Step 6)

### Step 6 — Create specs for each chunk

For each chunk, call `spec_create` with:
- `slug` — the chunk slug
- `name` — the chunk name
- `milestone_slug` — the milestone slug
- `criteria` — the text descriptions for this chunk

## Output Format

After all tools succeed, confirm what was created:

```
Created milestone: auth-layer (Auth Layer)
Specs created:
  - specs/login/gates.toml
  - specs/signup/gates.toml
  - specs/session-refresh/gates.toml

⚠ Next step: The generated gates.toml files contain text criteria only.
  Open each file and add a `cmd = "..."` field to any criterion you want
  to run as an executable gate. Without cmd, gates are treated as
  descriptive (not automatically verified).
```

Always include the warning about `cmd` manual editing — specs created this way require it before gates are runnable.
