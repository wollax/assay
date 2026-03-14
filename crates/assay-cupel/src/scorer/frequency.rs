use std::any::Any;

use crate::model::ContextItem;
use crate::scorer::Scorer;

/// Relative scorer: proportion of peers sharing at least one tag.
///
/// Self-exclusion uses reference identity (`std::ptr::eq`).
/// Tag comparison is case-insensitive (ASCII fold).
#[derive(Debug, Clone, Copy)]
pub struct FrequencyScorer;

impl Scorer for FrequencyScorer {
    fn score(&self, item: &ContextItem, all_items: &[ContextItem]) -> f64 {
        if item.tags().is_empty() || all_items.len() <= 1 {
            return 0.0;
        }

        let mut matching_items: usize = 0;

        for other in all_items {
            // Self-exclusion by reference identity
            if std::ptr::eq(item, other) {
                continue;
            }
            if other.tags().is_empty() {
                continue;
            }
            if shares_any_tag(item.tags(), other.tags()) {
                matching_items += 1;
            }
        }

        matching_items as f64 / (all_items.len() - 1) as f64
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Checks if two tag slices share any tag using case-insensitive ASCII comparison.
fn shares_any_tag(tags_a: &[String], tags_b: &[String]) -> bool {
    for a in tags_a {
        for b in tags_b {
            if a.eq_ignore_ascii_case(b) {
                return true;
            }
        }
    }
    false
}
