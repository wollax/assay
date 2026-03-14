mod classify;
mod deduplicate;
mod place;
mod score;
mod slice;
mod sort;

use crate::model::{ContextBudget, ContextItem, OverflowStrategy};
use crate::placer::Placer;
use crate::scorer::Scorer;
use crate::slicer::Slicer;
use crate::CupelError;

/// A fixed 6-stage pipeline that selects and orders context items within a token budget.
///
/// Stages execute in order: Classify, Score, Deduplicate, Sort, Slice, Place.
/// Stages cannot be reordered, skipped, or inserted between.
pub struct Pipeline {
    scorer: Box<dyn Scorer>,
    slicer: Box<dyn Slicer>,
    placer: Box<dyn Placer>,
    deduplication: bool,
    overflow_strategy: OverflowStrategy,
}

impl Pipeline {
    /// Creates a new `PipelineBuilder`.
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder {
            scorer: None,
            slicer: None,
            placer: None,
            deduplication: true,
            overflow_strategy: OverflowStrategy::default(),
        }
    }

    /// Executes the 6-stage pipeline on the given candidate items and budget.
    ///
    /// Stages: Classify -> Score -> Deduplicate -> Sort -> Slice -> Place
    pub fn run(
        &self,
        items: &[ContextItem],
        budget: &ContextBudget,
    ) -> Result<Vec<ContextItem>, CupelError> {
        // Stage 1: Classify
        let (pinned, scoreable) = classify::classify(items, budget)?;

        // Stage 2: Score
        let scored = score::score_items(&scoreable, self.scorer.as_ref());

        // Stage 3: Deduplicate
        let deduped = deduplicate::deduplicate(scored, self.deduplication);

        // Stage 4: Sort
        let sorted = sort::sort_scored(deduped);

        // Stage 5: Slice
        let pinned_tokens: i64 = pinned.iter().map(|i| i.tokens()).sum();
        let sliced =
            slice::slice_items(&sorted, budget, pinned_tokens, self.slicer.as_ref());

        // Stage 6: Place
        place::place_items(
            &pinned,
            &sliced,
            &sorted,
            budget,
            self.overflow_strategy,
            self.placer.as_ref(),
        )
    }
}

impl std::fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline")
            .field("deduplication", &self.deduplication)
            .field("overflow_strategy", &self.overflow_strategy)
            .finish()
    }
}

/// Builder for constructing a [`Pipeline`] with required and optional configuration.
pub struct PipelineBuilder {
    scorer: Option<Box<dyn Scorer>>,
    slicer: Option<Box<dyn Slicer>>,
    placer: Option<Box<dyn Placer>>,
    deduplication: bool,
    overflow_strategy: OverflowStrategy,
}

impl std::fmt::Debug for PipelineBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineBuilder")
            .field("scorer", &self.scorer.is_some())
            .field("slicer", &self.slicer.is_some())
            .field("placer", &self.placer.is_some())
            .field("deduplication", &self.deduplication)
            .field("overflow_strategy", &self.overflow_strategy)
            .finish()
    }
}

impl PipelineBuilder {
    /// Sets the scorer strategy (required).
    pub fn scorer(mut self, scorer: Box<dyn Scorer>) -> Self {
        self.scorer = Some(scorer);
        self
    }

    /// Sets the slicer strategy (required).
    pub fn slicer(mut self, slicer: Box<dyn Slicer>) -> Self {
        self.slicer = Some(slicer);
        self
    }

    /// Sets the placer strategy (required).
    pub fn placer(mut self, placer: Box<dyn Placer>) -> Self {
        self.placer = Some(placer);
        self
    }

    /// Enables or disables deduplication (default: enabled).
    pub fn deduplication(mut self, enabled: bool) -> Self {
        self.deduplication = enabled;
        self
    }

    /// Sets the overflow strategy (default: `Throw`).
    pub fn overflow_strategy(mut self, strategy: OverflowStrategy) -> Self {
        self.overflow_strategy = strategy;
        self
    }

    /// Builds the pipeline, validating that all required components are set.
    ///
    /// # Errors
    ///
    /// Returns an error if scorer, slicer, or placer is not set.
    pub fn build(self) -> Result<Pipeline, CupelError> {
        let scorer = self
            .scorer
            .ok_or_else(|| CupelError::PipelineConfig("scorer is required".to_owned()))?;
        let slicer = self
            .slicer
            .ok_or_else(|| CupelError::PipelineConfig("slicer is required".to_owned()))?;
        let placer = self
            .placer
            .ok_or_else(|| CupelError::PipelineConfig("placer is required".to_owned()))?;

        Ok(Pipeline {
            scorer,
            slicer,
            placer,
            deduplication: self.deduplication,
            overflow_strategy: self.overflow_strategy,
        })
    }
}
