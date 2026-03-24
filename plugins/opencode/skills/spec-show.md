---
name: spec-show
description: >
  Display a spec's criteria and details.
  Use when the user wants to see what a spec contains,
  what criteria need to be met, or before starting implementation.
---

# Spec Show

Display a spec's full definition including all criteria.

## Steps

1. **Determine which spec to show:**
   - If a spec name was provided as `$ARGUMENTS`, use that spec
   - If no spec was provided, call `spec_list` to show available specs and ask which one to display

2. **Fetch the spec:**
   - Call the `spec_get` tool with the spec name

3. **Present the spec:**
   - Show the spec name and description
   - List each criterion with:
     - Name
     - Description
     - Whether it's executable (has a `cmd`) or descriptive (no `cmd`)
     - The command that will be run (if executable)
     - Timeout override (if set)

## Output Format

Use a clear, structured format. Group criteria by type (executable vs descriptive). For executable criteria, show the exact command so the user knows what will run.
