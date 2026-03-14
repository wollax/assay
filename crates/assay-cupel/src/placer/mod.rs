mod chronological;
mod u_shaped;

pub use chronological::ChronologicalPlacer;
pub use u_shaped::UShapedPlacer;

use crate::model::{ContextItem, ScoredItem};

/// A placer determines the final presentation order of selected items.
///
/// Placers receive merged items (pinned + sliced) with their scores and
/// produce the final ordered list of context items.
pub trait Placer: Send + Sync {
    /// Orders the given items for final presentation in the context window.
    fn place(&self, items: &[ScoredItem]) -> Vec<ContextItem>;
}
