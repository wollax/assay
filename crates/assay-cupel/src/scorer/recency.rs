use std::any::Any;

use crate::model::ContextItem;
use crate::scorer::Scorer;

/// Rank-based scorer: more recent items score higher.
///
/// Items without a timestamp receive 0.0. A single timestamped item scores 1.0.
#[derive(Debug, Clone, Copy)]
pub struct RecencyScorer;

impl Scorer for RecencyScorer {
    fn score(&self, item: &ContextItem, all_items: &[ContextItem]) -> f64 {
        let item_ts = match item.timestamp() {
            Some(ts) => ts,
            None => return 0.0,
        };

        let mut count_with_timestamp: usize = 0;
        let mut rank: usize = 0;

        for other in all_items {
            if let Some(ts) = other.timestamp() {
                count_with_timestamp += 1;
                if ts < item_ts {
                    rank += 1;
                }
            }
        }

        if count_with_timestamp <= 1 {
            return 1.0;
        }

        rank as f64 / (count_with_timestamp - 1) as f64
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
