# Phase 44: gate_evaluate Context Budgeting - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire diff token budgeting into `gate_evaluate` so the evaluator subprocess receives a right-sized diff. Compute the budget as model context window minus spec/criteria/prompt overhead, truncate when the diff exceeds the budget, and capture truncation metadata in the gate run record. No new MCP tools — this enhances the existing `gate_evaluate` flow.

</domain>

<decisions>
## Implementation Decisions

### Budget calculation
- Model context window derived from the `--model` flag passed to the evaluator subprocess — look up known model windows from a mapping table
- Unknown models fall back to a conservative default window size and log a warning
- Token counting method for overhead (spec criteria, system prompt) is Claude's discretion — pick the most practical approach (char-based estimate, cupel tokenizer, etc.)
- Safety margin on the budget is Claude's discretion

### Truncation strategy
- Truncation approach (head-priority, head+tail split, etc.) is Claude's discretion — optimize for evaluator context quality
- Whether to respect file boundaries in diffs is Claude's discretion
- Whether to reuse existing `truncate_diff`/`truncate_head_tail` from Phase 29 or build new token-aware truncation is Claude's discretion
- Truncation marker format is Claude's discretion

### Truncation metadata
- Struct vs flat fields on GateRunRecord is Claude's discretion (follow existing codebase patterns)
- Truncation metadata is included in the MCP response **only when truncation occurred** (not when diff fits within budget)
- Metadata must include **file lists** — which files were included and which were omitted
- Truncation **always triggers a warning** in the MCP response via the Phase 35 `warnings` field (e.g., "Diff truncated from X to Y tokens (N files omitted)")

### Integration point
- Where budgeting happens in the gate_evaluate flow (before subprocess spawn vs inside evaluator module) is Claude's discretion
- Whether to use cupel/context-engine or build self-contained logic in assay-core is Claude's discretion
- If budgeting fails (cupel unavailable, errors), **graceful fallback** — pass full diff through and log a warning; gate_evaluate still works
- Budget computation logging level is Claude's discretion

### Claude's Discretion
- Token counting approach for overhead estimation
- Safety margin percentage (or none)
- Truncation strategy (head-only, head+tail split, proportions)
- File boundary respect in truncation
- Reuse of existing truncation infra vs new token-aware implementation
- Truncation marker format and content
- Struct design for truncation metadata on GateRunRecord
- Budgeting placement in the gate_evaluate flow
- cupel vs self-contained budgeting logic
- Log levels for budget computation trace

</decisions>

<specifics>
## Specific Ideas

- Success criteria #4 is explicit: when diff fits within budget, no truncation occurs and no truncation metadata is recorded — clean pass-through
- The model window lookup table should cover Claude model family at minimum (Haiku, Sonnet, Opus with their context windows)
- File lists in metadata let the calling agent understand exactly what the evaluator saw vs missed

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 44-gate-evaluate-context-budgeting*
*Context gathered: 2026-03-15*
