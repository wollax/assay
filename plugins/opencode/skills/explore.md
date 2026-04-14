# Explore

Load a lean project context and enter exploration mode.

## Steps

1. **Load tiered context (always, ~500 tokens):**
   - Call `cycle_status` — get active milestone/phase
   - Call `spec_list` — get spec index with names, criteria counts, and status
   - Summarize: project name, milestone status, spec overview

2. **Conditionally load (if active milestone exists):**
   - Call `chunk_status` for the active chunk — get gate pass/fail
   - Recent git activity: `git log --oneline -20`

3. **Present the summary** and ask: *"What would you like to explore?"*

4. **On-demand loading (when user asks):**
   - Full spec criteria: call `spec_get` for the requested spec
   - Gate run details: call `gate_history` with the spec name
   - Session data: call `session_list`

5. **Fresh project (no specs):**
   - Show config only
   - Ask: *"No specs defined yet. What are you building?"*

## Output Format

Keep the initial summary under 10 lines. Load details only when asked. Stay in conversation mode — help the user think, don't prescribe a workflow.
