# Phase 48: Gate Evidence Formatting - Context

**Gathered:** 2026-03-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Format gate results as markdown suitable for PR bodies with GitHub character limit handling. This phase produces a markdown formatter that takes gate run results and outputs structured evidence for PR bodies, plus writes a full report file to disk. It does NOT create PRs or interact with GitHub — that's Phase 50.

</domain>

<decisions>
## Implementation Decisions

### Markdown structure
- Summary + details layout: pass/fail summary table at top, expandable `<details>` sections below
- Detail sections for: all failed criteria AND agent-evaluated passes (deterministic passes are table-only)
- Failures are expanded by default, agent passes are collapsed

### Content density
- Agent evaluator reasoning is included in full (no summarization or condensing)
- Failed criteria rows are bold in the summary table; failure detail sections use bold or GitHub alert styling for visual distinction
- Deterministic criteria detail level: Claude's discretion

### Truncation strategy
- 65,536 character GitHub limit enforced
- Truncation priority, degradation order, and marker verbosity: Claude's discretion (optimize for reviewer usefulness)
- When truncated, include a reference to the full report file on disk
- Full report file path format: Claude's discretion

### Report file output
- This phase writes a full (untruncated) markdown report file to disk in addition to producing the PR body markdown
- The truncated PR body links to this file

### Claude's Discretion
- Whether to include spec name in the summary header
- Whether to show enforcement level (advisory/blocking) column in summary table
- Deterministic criteria detail level in expanded sections
- All-pass output format (full details vs compact)
- Zero-criteria output handling
- API boundary: whether formatter accepts run ID or pre-loaded results struct
- Evaluation-error state visual treatment (distinct icon vs failure)
- Truncation degradation priority and marker content
- Report file path and naming convention
- Footer metadata inclusion (run ID, timestamp, spec)

</decisions>

<specifics>
## Specific Ideas

- Bold treatment for failures — make them visually pop beyond just emoji
- Agent-evaluated passes include reasoning because it's valuable for reviewers to see what the agent checked
- Full reasoning preserved (not summarized) — truncation handles overflow, not content condensation

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 48-gate-evidence-formatting*
*Context gathered: 2026-03-16*
