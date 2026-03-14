/// Errors that can occur during Cupel pipeline construction and execution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CupelError {
    #[error("content must be non-empty")]
    EmptyContent,

    #[error("kind must be a non-empty, non-whitespace-only string")]
    EmptyKind,

    #[error("source must be a non-empty, non-whitespace-only string")]
    EmptySource,

    #[error("invalid budget: {0}")]
    InvalidBudget(String),

    #[error("pinned tokens ({pinned_tokens}) exceed available budget ({available})")]
    PinnedExceedsBudget { pinned_tokens: i64, available: i64 },

    #[error("merged tokens ({merged_tokens}) exceed target tokens ({target_tokens})")]
    Overflow {
        merged_tokens: i64,
        target_tokens: i64,
    },

    #[error("invalid pipeline configuration: {0}")]
    PipelineConfig(String),

    #[error("invalid scorer configuration: {0}")]
    ScorerConfig(String),

    #[error("invalid slicer configuration: {0}")]
    SlicerConfig(String),

    #[error("cycle detected: scorer appears in its own dependency graph")]
    CycleDetected,
}
