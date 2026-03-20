---
name: gate-check
description: >
  Run quality gates for a spec and report pass/fail results.
  Use when checking if code changes meet spec criteria,
  after implementing features, or when asked about gate status.
---

# Gate Check

Run quality gates and report structured results.

## Steps

1. **Determine which spec(s) to check:**
   - If a spec name was provided as `$ARGUMENTS`, use that spec
   - If no spec was provided, call the `spec_list` tool to discover all available specs, then run gates for each

2. **Run gates:**
   - Call the `gate_run` tool with the spec name
   - Omit `include_evidence` for the initial summary (defaults to false)

3. **Report results:**
   - **All passed:** Report concisely: "3/3 criteria passed for [spec-name]" with duration
   - **Any failed:** List each failed criterion with its `reason` field, then offer to show full evidence by calling `gate_run` with `include_evidence: true`

4. **If multiple specs:** Report results per-spec with an aggregate summary at the end

## Output Format

Keep output concise. For passing specs, one line is enough. For failures, show the criterion name, status, and failure reason. Only show full stdout/stderr evidence when explicitly requested or when the failure reason alone is insufficient to diagnose the issue.
