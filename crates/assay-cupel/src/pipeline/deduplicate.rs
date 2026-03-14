use std::collections::HashMap;

use crate::model::ScoredItem;

/// Removes items with duplicate content, keeping the highest-scoring instance.
///
/// Uses byte-exact ordinal comparison (no Unicode normalization, no case folding).
/// When scores are equal, the item with the lower original index wins.
/// If deduplication is disabled, returns the input unchanged.
pub(crate) fn deduplicate(scored: Vec<ScoredItem>, enabled: bool) -> Vec<ScoredItem> {
    if !enabled || scored.is_empty() {
        return scored;
    }

    // Map: content string -> index of best item
    let mut best_by_content: HashMap<String, usize> = HashMap::new();

    for (i, si) in scored.iter().enumerate() {
        let content = si.item.content().to_owned();
        match best_by_content.get(&content) {
            Some(&existing_idx) => {
                if si.score > scored[existing_idx].score {
                    best_by_content.insert(content, i);
                }
                // Equal score: keep lower index (existing), no action
            }
            None => {
                best_by_content.insert(content, i);
            }
        }
    }

    // Collect survivors in original order
    scored
        .into_iter()
        .enumerate()
        .filter(|(i, si)| best_by_content.get(si.item.content()) == Some(i))
        .map(|(_, si)| si)
        .collect()
}
