use std::any::Any;
use std::collections::HashSet;

use crate::CupelError;
use crate::model::ContextItem;
use crate::scorer::Scorer;

/// Composite scorer: weighted average of child scorers with normalized weights.
///
/// Validates at construction: at least one entry, all weights positive and finite,
/// and no cycles in the scorer graph (detected via DFS with reference identity).
pub struct CompositeScorer {
    scorers: Vec<Box<dyn Scorer>>,
    normalized_weights: Vec<f64>,
}

impl CompositeScorer {
    /// Creates a new CompositeScorer from a list of (scorer, weight) pairs.
    ///
    /// Weights are normalized to sum to 1.0.
    pub fn new(entries: Vec<(Box<dyn Scorer>, f64)>) -> Result<Self, CupelError> {
        if entries.is_empty() {
            return Err(CupelError::ScorerConfig(
                "at least one scorer entry is required".to_owned(),
            ));
        }

        let mut total_weight = 0.0;
        for (_, weight) in &entries {
            if *weight <= 0.0 {
                return Err(CupelError::ScorerConfig(
                    "weight must be positive".to_owned(),
                ));
            }
            if !weight.is_finite() {
                return Err(CupelError::ScorerConfig(
                    "weight must be finite".to_owned(),
                ));
            }
            total_weight += weight;
        }

        // Cycle detection via DFS with reference identity
        let mut visited: HashSet<usize> = HashSet::new();
        let mut in_path: HashSet<usize> = HashSet::new();
        for (scorer, _) in &entries {
            detect_cycles_dfs(scorer.as_ref(), &mut visited, &mut in_path)?;
        }

        let normalized_weights: Vec<f64> =
            entries.iter().map(|(_, w)| w / total_weight).collect();
        let scorers: Vec<Box<dyn Scorer>> = entries.into_iter().map(|(s, _)| s).collect();

        Ok(Self {
            scorers,
            normalized_weights,
        })
    }

    /// Returns references to child scorers (for cycle detection in outer composites).
    pub(crate) fn children(&self) -> &[Box<dyn Scorer>] {
        &self.scorers
    }
}

impl Scorer for CompositeScorer {
    fn score(&self, item: &ContextItem, all_items: &[ContextItem]) -> f64 {
        let mut result = 0.0;
        for i in 0..self.scorers.len() {
            result += self.scorers[i].score(item, all_items) * self.normalized_weights[i];
        }
        result
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for CompositeScorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeScorer")
            .field("num_scorers", &self.scorers.len())
            .field("normalized_weights", &self.normalized_weights)
            .finish()
    }
}

/// Extracts a stable identity key from a scorer reference for cycle detection.
/// Uses the data pointer of the trait object (ignoring vtable).
fn scorer_identity(scorer: &dyn Scorer) -> usize {
    scorer as *const dyn Scorer as *const () as usize
}

/// DFS cycle detection using reference identity via data pointer.
fn detect_cycles_dfs(
    node: &dyn Scorer,
    visited: &mut HashSet<usize>,
    in_path: &mut HashSet<usize>,
) -> Result<(), CupelError> {
    let id = scorer_identity(node);

    if visited.contains(&id) {
        if in_path.contains(&id) {
            return Err(CupelError::CycleDetected);
        }
        return Ok(());
    }

    visited.insert(id);
    in_path.insert(id);

    // Downcast to check for composite/scaled children
    if let Some(composite) = node.as_any().downcast_ref::<CompositeScorer>() {
        for child in composite.children() {
            detect_cycles_dfs(child.as_ref(), visited, in_path)?;
        }
    } else if let Some(scaled) = node.as_any().downcast_ref::<super::ScaledScorer>() {
        detect_cycles_dfs(scaled.inner(), visited, in_path)?;
    }

    in_path.remove(&id);
    Ok(())
}
