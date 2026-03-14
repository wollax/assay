use crate::model::{ContextBudget, ContextItem, ScoredItem};
use crate::slicer::Slicer;

/// Selects items by value density (score per token), greedily filling the budget
/// from highest-density items first.
///
/// Zero-token items have density `f64::MAX` and are always included.
pub struct GreedySlice;

impl Slicer for GreedySlice {
    fn slice(&self, sorted: &[ScoredItem], budget: &ContextBudget) -> Vec<ContextItem> {
        if sorted.is_empty() || budget.target_tokens() <= 0 {
            return Vec::new();
        }

        // Step 1: Compute value densities with original indices
        let mut densities: Vec<(f64, usize)> = sorted
            .iter()
            .enumerate()
            .map(|(i, si)| {
                let tokens = si.item.tokens();
                let density = if tokens == 0 {
                    f64::MAX
                } else {
                    si.score / tokens as f64
                };
                (density, i)
            })
            .collect();

        // Step 2: Stable sort by density descending, tiebreak by index ascending
        densities.sort_by(|a, b| {
            b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1))
        });

        // Step 3: Greedy fill
        let mut remaining = budget.target_tokens();
        let mut selected = Vec::new();

        for &(_, orig_idx) in &densities {
            let item = &sorted[orig_idx].item;
            let tokens = item.tokens();

            if tokens == 0 {
                selected.push(item.clone());
            } else if tokens <= remaining {
                selected.push(item.clone());
                remaining -= tokens;
            }
        }

        selected
    }
}
