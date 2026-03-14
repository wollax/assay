use crate::model::{ContextItem, ScoredItem};
use crate::placer::Placer;

/// Orders items by ascending timestamp, preserving natural temporal flow.
///
/// Timestamped items sort before null-timestamp items. Among timestamped items,
/// ordering is ascending by timestamp. Tiebreak is by original index (stable).
pub struct ChronologicalPlacer;

impl Placer for ChronologicalPlacer {
    fn place(&self, items: &[ScoredItem]) -> Vec<ContextItem> {
        if items.is_empty() {
            return Vec::new();
        }
        if items.len() == 1 {
            return vec![items[0].item.clone()];
        }

        // Build sortable array with original indices
        let mut sortable: Vec<usize> = (0..items.len()).collect();

        sortable.sort_by(|&a, &b| {
            let a_ts = items[a].item.timestamp();
            let b_ts = items[b].item.timestamp();

            match (a_ts, b_ts) {
                (Some(at), Some(bt)) => at.cmp(&bt).then_with(|| a.cmp(&b)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.cmp(&b),
            }
        });

        sortable
            .iter()
            .map(|&i| items[i].item.clone())
            .collect()
    }
}
