use std::collections::HashMap;

use crate::model::{ContextBudget, ContextItem, ScoredItem};
use crate::slicer::Slicer;

/// Computes the effective budget after accounting for pinned items and output reserve.
///
/// - `effectiveMax = max(0, maxTokens - outputReserve - pinnedTokens)`
/// - `effectiveTarget = min(max(0, targetTokens - pinnedTokens), effectiveMax)`
pub(crate) fn compute_effective_budget(
    budget: &ContextBudget,
    pinned_tokens: i64,
) -> ContextBudget {
    let effective_max =
        (budget.max_tokens() - budget.output_reserve() - pinned_tokens).max(0);
    let effective_target =
        (budget.target_tokens() - pinned_tokens).max(0).min(effective_max);

    // Create a minimal budget for the slicer — only maxTokens and targetTokens matter.
    ContextBudget::new(effective_max, effective_target, 0, HashMap::new(), 0.0)
        .expect("effective budget values are non-negative and target <= max")
}

/// Delegates to the slicer with the effective budget.
pub(crate) fn slice_items(
    sorted: &[ScoredItem],
    budget: &ContextBudget,
    pinned_tokens: i64,
    slicer: &dyn Slicer,
) -> Vec<ContextItem> {
    let adjusted = compute_effective_budget(budget, pinned_tokens);
    slicer.slice(sorted, &adjusted)
}
