use std::any::Any;

use crate::model::ContextItem;
use crate::scorer::Scorer;

/// Absolute scorer: passes through `futureRelevanceHint`, clamped to [0.0, 1.0].
///
/// Returns 0.0 for null or non-finite hints. Finiteness is checked before clamping.
#[derive(Debug, Clone, Copy)]
pub struct ReflexiveScorer;

impl Scorer for ReflexiveScorer {
    fn score(&self, item: &ContextItem, _all_items: &[ContextItem]) -> f64 {
        match item.future_relevance_hint() {
            None => 0.0,
            Some(value) => {
                if !value.is_finite() {
                    return 0.0;
                }
                value.clamp(0.0, 1.0)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
