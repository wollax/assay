use crate::model::ContextItem;

/// A value type associating a [`ContextItem`] with its computed relevance score.
#[derive(Debug, Clone)]
pub struct ScoredItem {
    /// The context item.
    pub item: ContextItem,
    /// The computed relevance score (conventionally in [0.0, 1.0]).
    pub score: f64,
}
