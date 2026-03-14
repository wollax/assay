use std::any::Any;
use std::collections::HashMap;

use crate::CupelError;
use crate::model::ContextItem;
use crate::scorer::Scorer;

/// Absolute scorer: weighted tag matching, normalized by total configured weight.
///
/// Tag key lookup is case-sensitive (ordinal) per spec.
#[derive(Debug, Clone)]
pub struct TagScorer {
    tag_weights: HashMap<String, f64>,
    total_weight: f64,
}

impl TagScorer {
    /// Creates a TagScorer with the given tag-weight map.
    ///
    /// Validates that all weights are non-negative and finite.
    pub fn new(tag_weights: HashMap<String, f64>) -> Result<Self, CupelError> {
        let mut total_weight = 0.0;
        for (tag, &weight) in &tag_weights {
            if weight < 0.0 {
                return Err(CupelError::ScorerConfig(format!(
                    "tag weight for '{}' must be non-negative",
                    tag,
                )));
            }
            if !weight.is_finite() {
                return Err(CupelError::ScorerConfig(format!(
                    "tag weight for '{}' must be finite",
                    tag,
                )));
            }
            total_weight += weight;
        }
        Ok(Self {
            tag_weights,
            total_weight,
        })
    }
}

impl Scorer for TagScorer {
    fn score(&self, item: &ContextItem, _all_items: &[ContextItem]) -> f64 {
        if self.total_weight == 0.0 || item.tags().is_empty() {
            return 0.0;
        }

        let mut matched_sum = 0.0;
        for tag in item.tags() {
            if let Some(&weight) = self.tag_weights.get(tag) {
                matched_sum += weight;
            }
        }

        f64::min(matched_sum / self.total_weight, 1.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
