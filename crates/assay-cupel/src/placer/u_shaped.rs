use crate::model::{ContextItem, ScoredItem};
use crate::placer::Placer;

/// Positions highest-scored items at both edges of the context window (start and
/// end), with lowest-scored items in the middle.
///
/// Exploits primacy and recency bias in LLMs by placing the most relevant items
/// where they receive the most attention.
pub struct UShapedPlacer;

impl Placer for UShapedPlacer {
    fn place(&self, items: &[ScoredItem]) -> Vec<ContextItem> {
        if items.is_empty() {
            return Vec::new();
        }
        if items.len() == 1 {
            return vec![items[0].item.clone()];
        }

        let n = items.len();

        // Step 1: Sort by score descending, tiebreak by index ascending
        let mut scored: Vec<(f64, usize)> = items
            .iter()
            .enumerate()
            .map(|(i, si)| (si.score, i))
            .collect();

        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

        // Step 2: Alternate placement — even ranks to left, odd ranks to right
        let mut result: Vec<Option<ContextItem>> = vec![None; n];
        let mut left = 0;
        let mut right = n - 1;

        for (rank, &(_, orig_idx)) in scored.iter().enumerate() {
            let item = items[orig_idx].item.clone();
            if rank % 2 == 0 {
                result[left] = Some(item);
                left += 1;
            } else {
                result[right] = Some(item);
                if right == 0 {
                    break;
                }
                right -= 1;
            }
        }

        result.into_iter().map(|o| o.expect("UShapedPlacer: all result slots must be filled")).collect()
    }
}
