use crate::model::ScoredItem;

/// Stable-sorts scored items by score descending, with index ascending as tiebreak.
pub(crate) fn sort_scored(deduped: Vec<ScoredItem>) -> Vec<ScoredItem> {
    if deduped.len() <= 1 {
        return deduped;
    }

    // Build indexed pairs, sort by composite key, reconstruct
    let mut indexed: Vec<(usize, ScoredItem)> = deduped.into_iter().enumerate().collect();

    indexed.sort_by(|(idx_a, a), (idx_b, b)| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| idx_a.cmp(idx_b))
    });

    indexed.into_iter().map(|(_, si)| si).collect()
}
