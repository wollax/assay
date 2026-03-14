use crate::model::{ContextBudget, ContextItem};
use crate::CupelError;

/// Partitions input items into pinned and scoreable lists, excluding invalid items.
///
/// Items with `tokens < 0` are excluded (checked before pinned status).
/// Validates that pinned items fit within `maxTokens - outputReserve`.
pub(crate) fn classify(
    items: &[ContextItem],
    budget: &ContextBudget,
) -> Result<(Vec<ContextItem>, Vec<ContextItem>), CupelError> {
    let mut pinned = Vec::new();
    let mut scoreable = Vec::new();

    for item in items {
        // Negative-token check BEFORE pinned check
        if item.tokens() < 0 {
            continue;
        }
        if item.pinned() {
            pinned.push(item.clone());
        } else {
            scoreable.push(item.clone());
        }
    }

    // Validate pinned budget
    let pinned_tokens: i64 = pinned.iter().map(|i| i.tokens()).sum();
    let available = budget.max_tokens() - budget.output_reserve();

    if pinned_tokens > available {
        return Err(CupelError::PinnedExceedsBudget {
            pinned_tokens,
            available,
        });
    }

    Ok((pinned, scoreable))
}
