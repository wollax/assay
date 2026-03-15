# Phase 39: Context Engine Integration - Research

**Researched:** 2026-03-14
**Confidence:** HIGH (all findings verified against cupel source code and assay codebase)

## Summary

Cupel v1.0.0 has a clean, stable API with builder patterns, trait-based strategies, and validated construction. Assay's gate evaluation flow has four distinct content sources (system prompt, spec criteria, spec description, diff) that map naturally to cupel's `ContextItem` model. The existing `crates/assay-cupel` is a stale copy of cupel's internals and must be deleted. The external cupel crate should be referenced via `path` dependency since it lives as a sibling repo with no git tags or crates.io publication.

## Standard Stack

| Component | Choice | Confidence |
|-----------|--------|------------|
| Context engine | `cupel` v1.0.0 (external crate, path dep) | HIGH |
| Dependency reference | `cupel = { path = "../cupel/crates/cupel" }` in workspace Cargo.toml | HIGH |
| Integration location | New `crates/assay-core/src/context/budgeting.rs` module | HIGH |
| Scorer | `PriorityScorer` (simple, uses explicit priority values) | HIGH |
| Slicer | `GreedySlice` (fills budget from highest-density items) | HIGH |
| Placer | `ChronologicalPlacer` (natural ordering for evaluator) | HIGH |
| Overflow strategy | `OverflowStrategy::Truncate` | HIGH |
| Token counting | Heuristic (reuse existing `estimate_tokens_from_bytes` at ~3.7 bytes/token) | HIGH |

### Rationale for Key Choices

**Path dependency:** Cupel has no git tags and is not on crates.io. Both repos live under `~/Git/personal/`. A path dep is the most practical choice. The memory file confirms "referenced via git tag/version" but the repo has no tags yet -- path is the pragmatic interim. This can be changed to a git tag or crates.io dep when cupel is published.

**PriorityScorer over KindScorer or CompositeScorer:** Assay's content sources have a clear, static priority hierarchy (criteria > system prompt > spec body > diff). Priority values on each `ContextItem` express this directly. KindScorer's default weights don't match assay's needs, and CompositeScorer adds unnecessary complexity for four items with fixed relative importance. PriorityScorer's rank-based normalization handles this cleanly.

**GreedySlice over KnapsackSlice or QuotaSlice:** With only four content categories and a clear priority ordering, greedy fill by value density is optimal. KnapsackSlice is for combinatorial optimization with many items. QuotaSlice is for enforcing per-kind budget partitions, which is unnecessary when criteria and system prompt are pinned.

**ChronologicalPlacer over UShapedPlacer:** The evaluator receives content in a single prompt. ChronologicalPlacer preserves the natural reading order (system prompt -> spec -> criteria -> diff). UShapedPlacer optimizes for primacy/recency bias in multi-turn conversations, which doesn't apply to a single evaluation prompt.

**OverflowStrategy::Truncate:** In gate_evaluate, the diff is the variable-size piece. If pinned items (criteria, system prompt) plus diff exceed budget, Truncate removes lowest-priority non-pinned items (the diff tail). Throw would fail the evaluation. Proceed would send over-budget content to the model, risking truncation by the API.

## Architecture Patterns

### Content Source Mapping

Four content sources map to cupel `ContextItem` instances:

| Assay Source | ContextKind | ContextSource | Pinned | Priority | Rationale |
|---|---|---|---|---|---|
| System prompt | `SystemPrompt` (well-known) | `Chat` | Yes | - | Must always be included; bypass scoring |
| Spec criteria | `Document` (well-known) | `Rag` | Yes | - | Evaluation instructions; must always be included |
| Spec body (name + description) | `Document` (well-known) | `Rag` | No | 80 | Context for evaluator; valuable but not essential |
| Diff output | Custom `"Diff"` kind | `Tool` | No | 50 | Variable size; primary truncation candidate |

**Criteria should be pinned** because they are the evaluation instructions the LLM must follow. If criteria are scored away, the evaluation is meaningless. System prompt is pinned for the same reason.

**Spec body as a single ContextItem:** Specs are typically small (name + description, a few hundred tokens). Splitting into sections adds complexity for no benefit. One item with kind=Document.

**Custom "Diff" kind:** Using a custom kind allows the scorer to differentiate diff from document content. `ContextKind::new("Diff")` is validated and works with cupel's case-insensitive comparison.

### Budget Calculation

```
max_tokens = model_context_window           (e.g., 200_000)
output_reserve = 4_096                      (structured JSON evaluator output)
target_tokens = max_tokens - output_reserve - safety_margin
safety_margin = target_tokens * 0.05        (5% for heuristic token counting error)
```

**Model context window source:** Use the existing `context_window_for_model()` from `assay-core/src/context/tokens.rs`, which returns 200,000 for all Claude models. This follows assay's existing pattern and keeps the single source of truth.

**Output reserve at 4,096:** The evaluator output is structured JSON with pass/fail, reasoning, and per-criterion results. 4K tokens is generous for this. Fixed constant is simpler than percentage and matches the predictable output size.

**Safety margin at 5%:** Heuristic token counting (~3.7 bytes/token) has ~10-15% error variance. A 5% safety margin provides a reasonable buffer without wasting significant budget. This aligns with cupel's `estimation_safety_margin_percent` field.

**Reserved slots: empty HashMap.** Pinning already guarantees criteria and system prompt inclusion. Reserved slots are for guaranteeing minimum items of a kind when using scored (non-pinned) items, which isn't needed here.

### Integration Function Signature

The integration surface is a single function in `assay-core::context::budgeting`:

```rust
/// Prepare context items for gate evaluation, applying token budget constraints.
///
/// Returns ordered content strings ready for prompt assembly.
/// When total content fits within budget, passes through without pipeline overhead.
pub fn budget_context(
    system_prompt: &str,
    spec_body: &str,
    criteria_text: &str,
    diff: &str,
    model_window: u64,
) -> Result<Vec<String>, AssayError>
```

**Passthrough when content fits:** Before constructing the pipeline, sum the heuristic token counts. If total <= target_tokens, return all content in order without running cupel. This avoids pipeline overhead in the common case where diffs are small.

**Always run cupel vs skip:** Skip when content fits. The pipeline adds allocation, sorting, and cloning overhead. For small diffs (the common case), passthrough is both faster and produces identical results.

### Deduplication: Disabled

Content sources in assay do not overlap -- system prompt, spec body, criteria, and diff are distinct inputs from different sources. Deduplication adds a content-comparison pass that would never match anything. Disable it via `PipelineBuilder::deduplication(false)`.

### Stale Crate Removal

The cleanup requires:
1. Delete `crates/assay-cupel/` directory entirely
2. Remove `assay-cupel = { path = "crates/assay-cupel" }` from root `Cargo.toml` workspace dependencies
3. Verify no other crate has `assay-cupel` as a dependency (confirmed: none do)

## Don't Hand-Roll

| Problem | Use Instead | Why |
|---------|-------------|-----|
| Token budget enforcement | Cupel pipeline | Already handles pinning, scoring, slicing, overflow |
| Token counting | `estimate_tokens_from_bytes()` in `context/tokens.rs` | Already exists, heuristic is sufficient for budgeting |
| Context window size lookup | `context_window_for_model()` in `context/tokens.rs` | Single source of truth for model limits |
| Content priority ordering | Cupel's `PriorityScorer` + `GreedySlice` | Trait-based, tested, handles edge cases |
| Overflow handling | Cupel's `OverflowStrategy::Truncate` | Removes lowest-priority non-pinned items automatically |

## Common Pitfalls

1. **Forgetting to remove `assay-cupel` from workspace members.** The root `Cargo.toml` uses `members = ["crates/*"]` glob, so deleting the directory is sufficient. But the `[workspace.dependencies]` line `assay-cupel = { path = "crates/assay-cupel" }` must also be removed, or `cargo` will error on the missing path.

2. **Token count type mismatch.** Cupel uses `i64` for token counts. Assay's existing `estimate_tokens_from_bytes` returns `u64`. The conversion must handle this (cast with bounds check or `as i64` given values will never exceed i64::MAX for realistic content).

3. **Empty diff edge case.** `truncate_diff` already handles empty strings by returning `(None, false, None)`. The budgeting function must handle the case where diff is empty -- create no ContextItem for it, or create one with 0 tokens. Creating none is cleaner.

4. **ContextKind::new() is fallible.** Custom kind `"Diff"` requires `ContextKind::new("Diff")` which returns `Result`. Handle the error or use a module-level constant initialized once.

5. **PipelineBuilder requires all three strategies.** Forgetting scorer, slicer, or placer causes a runtime error. The builder's `build()` validates this, but it's a runtime check not a compile-time guarantee.

6. **Path dependency portability.** `path = "../cupel/crates/cupel"` is relative to the workspace root. This works on the development machine but will break for other contributors or CI unless they also have cupel checked out at the sibling path. Document this requirement or switch to git dep when tags exist.

7. **Budget validation constraints.** `ContextBudget::new()` enforces `target_tokens <= max_tokens` and `output_reserve <= max_tokens`. The budget calculation must respect these or construction will fail. Calculate target after subtracting output_reserve to ensure the constraint holds.

## Code Examples

### Pipeline Construction

```rust
use cupel::{
    ContextBudget, ContextItem, ContextItemBuilder, ContextKind, ContextSource,
    OverflowStrategy, Pipeline, PriorityScorer, GreedySlice, ChronologicalPlacer,
};

fn build_pipeline() -> Result<Pipeline, cupel::CupelError> {
    Pipeline::builder()
        .scorer(Box::new(PriorityScorer))
        .slicer(Box::new(GreedySlice))
        .placer(Box::new(ChronologicalPlacer))
        .deduplication(false)
        .overflow_strategy(OverflowStrategy::Truncate)
        .build()
}
```

### Content Item Construction

```rust
fn build_items(
    system_prompt: &str,
    spec_body: &str,
    criteria_text: &str,
    diff: &str,
    estimate_tokens: impl Fn(&str) -> i64,
) -> Result<Vec<ContextItem>, cupel::CupelError> {
    let diff_kind = ContextKind::new("Diff")?;
    let mut items = Vec::with_capacity(4);

    // System prompt: pinned, always included
    items.push(
        ContextItemBuilder::new(system_prompt, estimate_tokens(system_prompt))
            .kind(ContextKind::new(ContextKind::SYSTEM_PROMPT)?)
            .pinned(true)
            .build()?
    );

    // Criteria: pinned, always included
    items.push(
        ContextItemBuilder::new(criteria_text, estimate_tokens(criteria_text))
            .kind(ContextKind::new(ContextKind::DOCUMENT)?)
            .source(ContextSource::new(ContextSource::RAG)?)
            .pinned(true)
            .build()?
    );

    // Spec body: scored, high priority
    if !spec_body.is_empty() {
        items.push(
            ContextItemBuilder::new(spec_body, estimate_tokens(spec_body))
                .kind(ContextKind::new(ContextKind::DOCUMENT)?)
                .source(ContextSource::new(ContextSource::RAG)?)
                .priority(80)
                .build()?
        );
    }

    // Diff: scored, lower priority, primary truncation target
    if !diff.is_empty() {
        items.push(
            ContextItemBuilder::new(diff, estimate_tokens(diff))
                .kind(diff_kind)
                .source(ContextSource::new(ContextSource::TOOL)?)
                .priority(50)
                .build()?
        );
    }

    Ok(items)
}
```

### Budget Construction

```rust
use std::collections::HashMap;

fn build_budget(model_window: u64) -> Result<ContextBudget, cupel::CupelError> {
    let output_reserve: i64 = 4_096;
    let max_tokens = model_window as i64;
    let usable = max_tokens - output_reserve;
    let safety = (usable as f64 * 0.05) as i64;
    let target_tokens = usable - safety;

    ContextBudget::new(
        max_tokens,
        target_tokens,
        output_reserve,
        HashMap::new(),  // no reserved slots; pinning handles guarantees
        5.0,             // 5% safety margin
    )
}
```

### Passthrough Check

```rust
fn should_passthrough(contents: &[&str], target_tokens: i64) -> bool {
    let total: i64 = contents.iter()
        .map(|s| estimate_tokens_from_bytes(s.len() as u64) as i64)
        .sum();
    total <= target_tokens
}
```

## Sources

| Source | What | Confidence |
|--------|------|------------|
| `/Users/wollax/Git/personal/cupel/crates/cupel/src/` | Full cupel v1.0.0 source code | HIGH |
| `/Users/wollax/Git/personal/cupel/crates/cupel/Cargo.toml` | Cupel package metadata and deps | HIGH |
| `/Users/wollax/Git/personal/assay/Cargo.toml` | Assay workspace structure | HIGH |
| `/Users/wollax/Git/personal/assay/crates/assay-core/src/gate/mod.rs` | Gate evaluation flow | HIGH |
| `/Users/wollax/Git/personal/assay/crates/assay-core/src/context/tokens.rs` | Token estimation utilities | HIGH |
| `/Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs` | Spec, Config, GatesConfig types | HIGH |
| `/Users/wollax/Git/personal/assay/crates/assay-types/src/criterion.rs` | Criterion type with prompt field | HIGH |
| `/Users/wollax/Git/personal/assay/crates/assay-cupel/` | Stale prototype (to be removed) | HIGH |

## Metadata

- **Research method:** Direct source code examination of both cupel and assay codebases
- **Verification level:** All findings verified against actual source files
- **Open questions:** None -- cupel's API is stable and well-understood
- **Blocked on:** Nothing -- cupel v1.0.0 is ready for integration

---

*Phase: 39-context-engine-integration*
*Research completed: 2026-03-14*
