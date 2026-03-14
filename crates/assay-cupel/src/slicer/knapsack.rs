use crate::model::{ContextBudget, ContextItem, ScoredItem};
use crate::slicer::Slicer;
use crate::CupelError;

/// Selects items using 0/1 knapsack dynamic programming, finding the combination
/// that maximizes total score within the token budget.
///
/// Discretizes weights and capacity using a configurable bucket size for
/// feasible DP table sizes.
pub struct KnapsackSlice {
    bucket_size: i64,
}

impl KnapsackSlice {
    /// Creates a new `KnapsackSlice` with the given bucket size.
    ///
    /// # Errors
    ///
    /// Returns `CupelError::SlicerConfig` if `bucket_size` is not positive.
    pub fn new(bucket_size: i64) -> Result<Self, CupelError> {
        if bucket_size <= 0 {
            return Err(CupelError::SlicerConfig(
                "bucket_size must be > 0".to_owned(),
            ));
        }
        Ok(Self { bucket_size })
    }

    /// Creates a new `KnapsackSlice` with the default bucket size of 100.
    pub fn with_default_bucket_size() -> Self {
        Self { bucket_size: 100 }
    }
}

impl Slicer for KnapsackSlice {
    fn slice(&self, sorted: &[ScoredItem], budget: &ContextBudget) -> Vec<ContextItem> {
        if sorted.is_empty() || budget.target_tokens() <= 0 {
            return Vec::new();
        }

        // Step 1: Pre-filter zero-token items (always included)
        let mut zero_token_items: Vec<ContextItem> = Vec::new();
        let mut candidates: Vec<&ScoredItem> = Vec::new();

        for si in sorted {
            if si.item.tokens() == 0 {
                zero_token_items.push(si.item.clone());
            } else if si.item.tokens() > 0 {
                candidates.push(si);
            }
        }

        if candidates.is_empty() {
            return zero_token_items;
        }

        let n = candidates.len();

        // Step 2: Build parallel arrays
        let mut weights = Vec::with_capacity(n);
        let mut values = Vec::with_capacity(n);

        for c in &candidates {
            weights.push(c.item.tokens());
            let v = (c.score * 10000.0).floor() as i64;
            values.push(v.max(0));
        }

        // Step 3: Discretize capacity and weights
        let capacity = (budget.target_tokens() / self.bucket_size) as usize;
        if capacity == 0 {
            return zero_token_items;
        }

        let discretized_weights: Vec<usize> = weights
            .iter()
            .map(|&w| ((w + self.bucket_size - 1) / self.bucket_size) as usize)
            .collect();

        // Step 4: DP with 1D value array + 2D keep table
        let mut dp = vec![0i64; capacity + 1];
        let mut keep = vec![vec![false; capacity + 1]; n];

        for i in 0..n {
            let dw = discretized_weights[i];
            let dv = values[i];
            for w in (dw..=capacity).rev() {
                let with_item = dp[w - dw] + dv;
                if with_item > dp[w] {
                    dp[w] = with_item;
                    keep[i][w] = true;
                }
            }
        }

        // Step 5: Reconstruct solution
        let mut selected: Vec<ContextItem> = Vec::new();
        let mut remaining_capacity = capacity;

        for i in (0..n).rev() {
            if keep[i][remaining_capacity] {
                selected.push(candidates[i].item.clone());
                remaining_capacity -= discretized_weights[i];
            }
        }

        // Step 6: Combine zero-token items with selected items
        let mut result = zero_token_items;
        result.extend(selected);
        result
    }
}
