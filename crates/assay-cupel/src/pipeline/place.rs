use crate::model::{ContextItem, ContextBudget, OverflowStrategy, ScoredItem};
use crate::placer::Placer;
use crate::CupelError;

/// Merges pinned items with sliced items, handles overflow, and delegates to
/// the placer for final ordering.
///
/// Pinned items are assigned score 1.0. Overflow detection compares against the
/// ORIGINAL `targetTokens` (not the effective budget used for slicing).
pub(crate) fn place_items(
    pinned: &[ContextItem],
    sliced: &[ContextItem],
    sorted_scored: &[ScoredItem],
    budget: &ContextBudget,
    overflow_strategy: OverflowStrategy,
    placer: &dyn Placer,
) -> Result<Vec<ContextItem>, CupelError> {
    // Step 1: Merge — pinned items with score 1.0, then sliced items with original scores
    let mut merged: Vec<ScoredItem> = Vec::with_capacity(pinned.len() + sliced.len());

    for item in pinned {
        merged.push(ScoredItem {
            item: item.clone(),
            score: 1.0,
        });
    }

    for item in sliced {
        // Look up the original score from sorted_scored by content match
        let score = sorted_scored
            .iter()
            .find(|si| si.item.content() == item.content())
            .map_or(0.0, |si| si.score);

        merged.push(ScoredItem {
            item: item.clone(),
            score,
        });
    }

    // Step 2: Overflow detection — compare against ORIGINAL targetTokens
    let merged_tokens: i64 = merged.iter().map(|si| si.item.tokens()).sum();

    if merged_tokens > budget.target_tokens() {
        merged = handle_overflow(merged, budget.target_tokens(), overflow_strategy)?;
    }

    // Step 3: Delegate to placer for final ordering
    Ok(placer.place(&merged))
}

fn handle_overflow(
    merged: Vec<ScoredItem>,
    target_tokens: i64,
    strategy: OverflowStrategy,
) -> Result<Vec<ScoredItem>, CupelError> {
    match strategy {
        OverflowStrategy::Throw => {
            let merged_tokens: i64 = merged.iter().map(|si| si.item.tokens()).sum();
            Err(CupelError::Overflow {
                merged_tokens,
                target_tokens,
            })
        }
        OverflowStrategy::Truncate => {
            let mut kept = Vec::new();
            let mut current_tokens: i64 = 0;

            for si in merged {
                let fits = si.item.pinned()
                    || current_tokens + si.item.tokens() <= target_tokens;
                if fits {
                    current_tokens += si.item.tokens();
                    kept.push(si);
                }
            }

            Ok(kept)
        }
        OverflowStrategy::Proceed => {
            // Accept over-budget selection
            Ok(merged)
        }
    }
}
