use std::any::Any;

use crate::model::ContextItem;
use crate::scorer::Scorer;

/// Composite scorer: min-max normalization of an inner scorer across all items.
///
/// Returns 0.5 for degenerate cases (empty allItems, all scores equal).
/// Self-identification uses reference identity (`std::ptr::eq`).
pub struct ScaledScorer {
    inner: Box<dyn Scorer>,
}

impl ScaledScorer {
    /// Creates a new ScaledScorer wrapping the given inner scorer.
    pub fn new(inner: Box<dyn Scorer>) -> Self {
        Self { inner }
    }

    /// Returns a reference to the inner scorer (for cycle detection).
    pub(crate) fn inner(&self) -> &dyn Scorer {
        self.inner.as_ref()
    }
}

impl Scorer for ScaledScorer {
    fn score(&self, item: &ContextItem, all_items: &[ContextItem]) -> f64 {
        if all_items.is_empty() {
            return 0.5;
        }

        let mut raw_score = f64::NAN;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for i in 0..all_items.len() {
            let s = self.inner.score(&all_items[i], all_items);
            if s < min {
                min = s;
            }
            if s > max {
                max = s;
            }
            if std::ptr::eq(item, &all_items[i]) {
                raw_score = s;
            }
        }

        debug_assert!(raw_score.is_finite(), "ScaledScorer: item not found in all_items via reference identity");

        if max == min {
            return 0.5;
        }

        (raw_score - min) / (max - min)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for ScaledScorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScaledScorer").finish()
    }
}
