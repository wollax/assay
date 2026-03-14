use crate::model::{ContextItem, ScoredItem};
use crate::scorer::Scorer;

/// Computes a relevance score for each scoreable item using the configured scorer.
///
/// For each item, calls `scorer.score(item, all_items)` and wraps the result
/// in a `ScoredItem`. Output order matches input order.
pub(crate) fn score_items(scoreable: &[ContextItem], scorer: &dyn Scorer) -> Vec<ScoredItem> {
    let mut scored = Vec::with_capacity(scoreable.len());

    for item in scoreable {
        let score = scorer.score(item, scoreable);
        scored.push(ScoredItem {
            item: item.clone(),
            score,
        });
    }

    scored
}
