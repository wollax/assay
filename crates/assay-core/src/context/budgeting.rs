//! Context budgeting: maps assay content sources to cupel's pipeline model.
//!
//! The [`budget_context`] function is the integration surface between assay and cupel.
//! It handles the common case (small content, passthrough) efficiently while correctly
//! truncating large diffs via cupel's pipeline when the budget is exceeded.

use std::collections::HashMap;

use cupel::{
    ChronologicalPlacer, ContextBudget, ContextItemBuilder, ContextKind, ContextSource,
    GreedySlice, OverflowStrategy, Pipeline, PriorityScorer,
};

use super::tokens::estimate_tokens_from_bytes;
use crate::AssayError;

/// Output reserve: tokens reserved for model output generation.
const OUTPUT_RESERVE: i64 = 4_096;

/// Safety margin: percentage buffer for token estimation error.
const SAFETY_MARGIN_PERCENT: f64 = 5.0;

/// Prepare token-budgeted context from assay's content sources.
///
/// Returns an ordered list of content strings that fit within the token budget
/// derived from `model_window`. The order is:
/// system prompt, spec body (if non-empty), criteria text, diff (if non-empty).
///
/// When total content fits within the budget, content passes through without
/// pipeline overhead. When the budget is exceeded, the diff is the primary
/// truncation target while system prompt and criteria are always included.
pub fn budget_context(
    system_prompt: &str,
    spec_body: &str,
    criteria_text: &str,
    diff: &str,
    model_window: u64,
) -> Result<Vec<String>, AssayError> {
    let max_tokens = model_window as i64;
    let usable = max_tokens - OUTPUT_RESERVE;
    let safety = (usable as f64 * (SAFETY_MARGIN_PERCENT / 100.0)) as i64;
    let target_tokens = usable - safety;

    // Collect non-empty inputs with their token estimates.
    let mut parts: Vec<(&str, i64)> = Vec::with_capacity(4);

    let prompt_tokens = estimate_tokens_from_bytes(system_prompt.len() as u64) as i64;
    if !system_prompt.is_empty() {
        parts.push((system_prompt, prompt_tokens));
    }

    let spec_tokens = estimate_tokens_from_bytes(spec_body.len() as u64) as i64;
    if !spec_body.is_empty() {
        parts.push((spec_body, spec_tokens));
    }

    let criteria_tokens = estimate_tokens_from_bytes(criteria_text.len() as u64) as i64;
    if !criteria_text.is_empty() {
        parts.push((criteria_text, criteria_tokens));
    }

    let diff_tokens = estimate_tokens_from_bytes(diff.len() as u64) as i64;
    if !diff.is_empty() {
        parts.push((diff, diff_tokens));
    }

    if parts.is_empty() {
        return Ok(Vec::new());
    }

    // Passthrough: if everything fits, return content in order without pipeline overhead.
    let total_tokens: i64 = parts.iter().map(|(_, t)| t).sum();
    if total_tokens <= target_tokens {
        return Ok(parts.into_iter().map(|(s, _)| s.to_owned()).collect());
    }

    // Pipeline path: build cupel items and run the pipeline.
    let map_err = |e: cupel::CupelError| AssayError::ContextBudget {
        message: e.to_string(),
    };

    let mut items = Vec::with_capacity(4);

    if !system_prompt.is_empty() {
        items.push(
            ContextItemBuilder::new(system_prompt, prompt_tokens)
                .kind(ContextKind::new(ContextKind::SYSTEM_PROMPT).map_err(map_err)?)
                .pinned(true)
                .build()
                .map_err(map_err)?,
        );
    }

    if !criteria_text.is_empty() {
        items.push(
            ContextItemBuilder::new(criteria_text, criteria_tokens)
                .kind(ContextKind::new(ContextKind::DOCUMENT).map_err(map_err)?)
                .source(ContextSource::new(ContextSource::RAG).map_err(map_err)?)
                .pinned(true)
                .build()
                .map_err(map_err)?,
        );
    }

    if !spec_body.is_empty() {
        items.push(
            ContextItemBuilder::new(spec_body, spec_tokens)
                .kind(ContextKind::new(ContextKind::DOCUMENT).map_err(map_err)?)
                .source(ContextSource::new(ContextSource::RAG).map_err(map_err)?)
                .priority(80)
                .build()
                .map_err(map_err)?,
        );
    }

    if !diff.is_empty() {
        items.push(
            ContextItemBuilder::new(diff, diff_tokens)
                .kind(ContextKind::new("Diff").map_err(map_err)?)
                .source(ContextSource::new(ContextSource::TOOL).map_err(map_err)?)
                .priority(50)
                .build()
                .map_err(map_err)?,
        );
    }

    let pipeline = Pipeline::builder()
        .scorer(Box::new(PriorityScorer))
        .slicer(Box::new(GreedySlice))
        .placer(Box::new(ChronologicalPlacer))
        .deduplication(false)
        .overflow_strategy(OverflowStrategy::Truncate)
        .build()
        .map_err(map_err)?;

    let budget = ContextBudget::new(
        max_tokens,
        target_tokens,
        OUTPUT_RESERVE,
        HashMap::new(),
        SAFETY_MARGIN_PERCENT,
    )
    .map_err(map_err)?;

    let result = pipeline.run(&items, &budget).map_err(map_err)?;

    Ok(result
        .into_iter()
        .map(|item| item.content().to_owned())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_content_fits() {
        let result = budget_context("prompt", "spec body", "criteria", "small diff", 200_000)
            .expect("should succeed");

        assert_eq!(result.len(), 4);
        assert_eq!(result[0], "prompt");
        assert_eq!(result[1], "spec body");
        assert_eq!(result[2], "criteria");
        assert_eq!(result[3], "small diff");
    }

    #[test]
    fn passthrough_skips_empty_diff() {
        let result =
            budget_context("prompt", "spec body", "criteria", "", 200_000).expect("should succeed");

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "prompt");
        assert_eq!(result[1], "spec body");
        assert_eq!(result[2], "criteria");
    }

    #[test]
    fn passthrough_skips_empty_spec_body() {
        let result =
            budget_context("prompt", "", "criteria", "diff", 200_000).expect("should succeed");

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "prompt");
        assert_eq!(result[1], "criteria");
        assert_eq!(result[2], "diff");
    }

    #[test]
    fn truncates_large_diff() {
        let large_diff = "x".repeat(1_000_000);
        let result = budget_context("prompt", "spec", "criteria", &large_diff, 200_000)
            .expect("should succeed");

        // The result should have fewer total bytes than the input diff alone
        let total_bytes: usize = result.iter().map(|s| s.len()).sum();
        assert!(
            total_bytes < large_diff.len(),
            "total output ({total_bytes}) should be less than input diff ({})",
            large_diff.len()
        );
    }

    #[test]
    fn pinned_items_always_included() {
        let large_diff = "x".repeat(1_000_000);
        let result = budget_context(
            "system prompt text",
            "spec",
            "criteria text",
            &large_diff,
            200_000,
        )
        .expect("should succeed");

        // System prompt and criteria must always be present (they are pinned)
        assert!(
            result.iter().any(|s| s == "system prompt text"),
            "system prompt must be included in result: {result:?}"
        );
        assert!(
            result.iter().any(|s| s == "criteria text"),
            "criteria must be included in result: {result:?}"
        );
    }

    #[test]
    fn empty_everything_returns_empty() {
        let result = budget_context("", "", "", "", 200_000).expect("should succeed");
        assert!(result.is_empty(), "all empty inputs should yield empty vec");
    }

    #[test]
    fn budget_calculation_correctness() {
        // For a 200k window:
        // max_tokens = 200_000
        // usable = 200_000 - 4_096 = 195_904
        // safety = floor(195_904 * 0.05) = floor(9_795.2) = 9_795
        // target_tokens = 195_904 - 9_795 = 186_109
        let max_tokens: i64 = 200_000;
        let usable = max_tokens - OUTPUT_RESERVE;
        let safety = (usable as f64 * (SAFETY_MARGIN_PERCENT / 100.0)) as i64;
        let target = usable - safety;

        assert_eq!(usable, 195_904);
        assert_eq!(safety, 9_795);
        assert_eq!(target, 186_109);
    }
}
