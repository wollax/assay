pub mod error;
pub mod model;
pub mod placer;
pub mod pipeline;
pub mod scorer;
pub mod slicer;

pub use error::CupelError;
pub use model::{
    ContextBudget, ContextItem, ContextItemBuilder, ContextKind, ContextSource, OverflowStrategy,
    ScoredItem,
};
pub use placer::{ChronologicalPlacer, Placer, UShapedPlacer};
pub use pipeline::{Pipeline, PipelineBuilder};
pub use scorer::{
    CompositeScorer, FrequencyScorer, KindScorer, PriorityScorer, RecencyScorer, ReflexiveScorer,
    ScaledScorer, Scorer, TagScorer,
};
pub use slicer::{GreedySlice, KnapsackSlice, QuotaEntry, QuotaSlice, Slicer};
