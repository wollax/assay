use std::any::Any;
use std::collections::HashMap;

use crate::CupelError;
use crate::model::{ContextItem, ContextKind};
use crate::scorer::Scorer;

/// Absolute scorer: assigns a score based on the item's kind using a weight map.
///
/// Weight lookup is case-insensitive (via ContextKind's Hash/Eq).
#[derive(Debug, Clone)]
pub struct KindScorer {
    weights: HashMap<ContextKind, f64>,
}

impl KindScorer {
    /// Creates a KindScorer with the default weight map.
    ///
    /// Default weights: SystemPrompt=1.0, Memory=0.8, ToolOutput=0.6, Document=0.4, Message=0.2
    pub fn with_default_weights() -> Self {
        let mut weights = HashMap::new();
        weights.insert(
            ContextKind::from_static(ContextKind::SYSTEM_PROMPT),
            1.0,
        );
        weights.insert(ContextKind::from_static(ContextKind::MEMORY), 0.8);
        weights.insert(
            ContextKind::from_static(ContextKind::TOOL_OUTPUT),
            0.6,
        );
        weights.insert(ContextKind::from_static(ContextKind::DOCUMENT), 0.4);
        weights.insert(ContextKind::from_static(ContextKind::MESSAGE), 0.2);
        Self { weights }
    }

    /// Creates a KindScorer with custom weights.
    ///
    /// Validates that all weights are non-negative and finite.
    pub fn new(weights: HashMap<ContextKind, f64>) -> Result<Self, CupelError> {
        for (kind, &weight) in &weights {
            if weight < 0.0 {
                return Err(CupelError::ScorerConfig(format!(
                    "weight for kind '{}' must be non-negative",
                    kind,
                )));
            }
            if !weight.is_finite() {
                return Err(CupelError::ScorerConfig(format!(
                    "weight for kind '{}' must be finite",
                    kind,
                )));
            }
        }
        Ok(Self { weights })
    }
}

impl Scorer for KindScorer {
    fn score(&self, item: &ContextItem, _all_items: &[ContextItem]) -> f64 {
        self.weights.get(item.kind()).copied().unwrap_or(0.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
