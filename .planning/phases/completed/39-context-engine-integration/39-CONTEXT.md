# Phase 39: Context Engine Integration - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire the external cupel crate (v1.0.0) into assay's workspace as the context budgeting engine. Define how assay's content sources (diff, spec, criteria, system prompt) map to cupel's pipeline model. Implement fallback when content fits without truncation. Remove the stale `crates/assay-cupel` prototype.

Cupel's API is stable at 1.0.0 with: `Pipeline` (6-stage: Classify, Score, Deduplicate, Sort, Slice, Place), `ContextItem` (content, tokens, kind, source, priority, tags, pinned, etc.), `ContextBudget` (max_tokens, target_tokens, output_reserve, reserved_slots, safety_margin), trait-based Scorer/Slicer/Placer strategies, and three OverflowStrategies (Throw, Truncate, Proceed).

</domain>

<decisions>
## Implementation Decisions

### Content Source Mapping
- Claude's discretion on which `ContextKind` to use for each assay content source (diff, spec body, criteria, system prompt) — custom kinds like "Diff" are available if needed for scoring differentiation
- Claude's discretion on whether spec body is a single ContextItem or split into sections — consider typical spec sizes
- Claude's discretion on whether criteria are pinned (always included) or scored — consider that criteria are the evaluation instructions
- Claude's discretion on token counting approach (heuristic vs real tokenizer) — weigh accuracy needs against dependency cost

### Pipeline Configuration
- Claude's discretion on overflow strategy (Truncate, Proceed, or Throw) — consider the gate_evaluate flow where diff is the variable-size piece
- Claude's discretion on hardcoded vs per-spec pipeline config — follow YAGNI, can extract config later
- Claude's discretion on deduplication enabled/disabled — analyze whether content sources can overlap
- Claude's discretion on placement strategy (Chronological vs U-shaped) — optimize for evaluator quality

### Budget Calculation
- Claude's discretion on where model context window size comes from — follow assay's existing config cascade patterns
- Claude's discretion on output_reserve approach (fixed constant vs percentage) — evaluator output is structured JSON
- Claude's discretion on reserved_slots usage — consider interaction with pinning strategy
- Claude's discretion on safety margin percentage — align with token counting accuracy

### Dependency Wiring
- **Remove `crates/assay-cupel`** — stale in-repo prototype, replaced by external cupel crate
- Claude's discretion on dependency reference method (git tag, path, crates.io) — pick the most practical for the current dev setup
- Claude's discretion on where integration code lives (assay-core module vs adapter crate) — consider the dependency graph
- Claude's discretion on error handling (fail gate_evaluate vs fallback to passthrough) — consider error severity
- Claude's discretion on whether cupel always runs or is skipped when content fits — weigh consistency vs performance

### Claude's Discretion
All implementation decisions are delegated to Claude. The user trusts Claude to make optimal choices based on:
- Cupel's actual API surface (examined during this discussion)
- Assay's existing patterns (config cascade, error handling, module organization)
- YAGNI principle — hardcode first, extract config from usage
- The gate_evaluate use case as the primary consumer

</decisions>

<specifics>
## Specific Ideas

- Cupel is at `/Users/wollax/Git/personal/cupel` — sibling repo to assay
- Cupel is published/packaged at v1.0.0 (see `crates/cupel/target/package/cupel-1.0.0/`)
- The stale `crates/assay-cupel` contains an older copy of cupel's types (model, scorer, slicer, placer, pipeline modules) — must be fully removed including workspace Cargo.toml references
- Cupel's well-known ContextKinds: Message, Document, ToolOutput, Memory, SystemPrompt
- Cupel's well-known ContextSources: Chat, Tool, Rag
- ContextItem supports `pinned: bool` to bypass scoring/slicing
- Pipeline requires all three strategies: Scorer + Slicer + Placer

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 39-context-engine-integration*
*Context gathered: 2026-03-14*
