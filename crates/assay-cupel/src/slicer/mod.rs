mod greedy;
mod knapsack;
mod quota;

pub use greedy::GreedySlice;
pub use knapsack::KnapsackSlice;
pub use quota::{QuotaEntry, QuotaSlice};

use crate::model::{ContextBudget, ContextItem, ScoredItem};

/// A slicer selects items from a sorted list to fit within a token budget.
///
/// Slicers receive items pre-sorted by score descending (from the Sort stage)
/// and return the subset of items that fits within the budget.
pub trait Slicer: Send + Sync {
    /// Selects items from `sorted` that fit within `budget`.
    fn slice(&self, sorted: &[ScoredItem], budget: &ContextBudget) -> Vec<ContextItem>;
}
