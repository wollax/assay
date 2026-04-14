> **Deprecated:** This skill has been replaced by `/assay:focus` (for status/next-chunk) or `/assay:check` (for gate-check). The old name still works but will be removed in the next version.

# Check

Run quality gates with smart routing and suggest next steps.

## Steps

1. **Determine which spec to check:**
   - If a spec name was provided as `$ARGUMENTS`, use that
   - Otherwise, call `cycle_status` to find the active chunk and use its slug
   - If no active work: tell the user and suggest `/assay:explore` or `/assay:plan`

2. **Run gates:**
   - Call `gate_run` with the spec name
   - Set `include_evidence` to `false` for the initial summary

3. **Report results:**
   - **All required passed:** Report concisely with duration
   - **Any required failed:** List each failed criterion with reason, offer to show evidence
   - **Pipeline-only criteria** (EventCount, NoToolErrors): note as skipped (require agent session)

4. **Suggest next step:**
   - All passed → suggest `/assay:ship` or advancing to next chunk
   - Failures → suggest fixes based on failure reasons
   - Draft spec → suggest reviewing/approving before shipping

## Output Format

One-line summary for passing specs. For failures, show criterion name and failure reason. End with a concrete next-step suggestion.
