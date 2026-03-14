mod composite;
mod frequency;
mod kind;
mod priority;
mod recency;
mod reflexive;
mod scaled;
mod tag;

pub use composite::CompositeScorer;
pub use frequency::FrequencyScorer;
pub use kind::KindScorer;
pub use priority::PriorityScorer;
pub use recency::RecencyScorer;
pub use reflexive::ReflexiveScorer;
pub use scaled::ScaledScorer;
pub use tag::TagScorer;

use std::any::Any;

use crate::model::ContextItem;

/// A scorer computes a relevance score for a context item relative to all scoreable items.
///
/// Scorers are pure functions: given identical inputs, they must return the same output.
/// Scores are conventionally in [0.0, 1.0] but this is not enforced.
pub trait Scorer: Any + Send + Sync {
    /// Computes a relevance score for `item` given the full list of scoreable items.
    fn score(&self, item: &ContextItem, all_items: &[ContextItem]) -> f64;

    /// Returns `self` as `&dyn Any` for downcasting (used by cycle detection).
    #[doc(hidden)]
    fn as_any(&self) -> &dyn Any;
}
