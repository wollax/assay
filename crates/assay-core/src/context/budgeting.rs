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

/// Priority for spec body: high, because the evaluator needs spec context to assess criteria.
const PRIORITY_SPEC: i64 = 80;

/// Priority for diff: lower than spec, making it the primary truncation target when budget is tight.
const PRIORITY_DIFF: i64 = 50;

/// Estimate tokens for a string, returning i64 for cupel compatibility.
fn estimate_tokens(s: &str) -> i64 {
    estimate_tokens_from_bytes(s.len() as u64) as i64
}

/// Prepare token-budgeted context from assay's content sources.
///
/// Returns an ordered list of content strings that fit within the token budget
/// derived from `model_window`. The order is:
/// system prompt, spec body (if non-empty), criteria text (if non-empty),
/// diff (if non-empty).
///
/// When total content fits within the budget, content passes through without
/// pipeline overhead. When the budget is exceeded, the diff is the primary
/// truncation target while system prompt and criteria are pinned (always
/// included when non-empty).
pub fn budget_context(
    system_prompt: &str,
    spec_body: &str,
    criteria_text: &str,
    diff: &str,
    model_window: u64,
) -> Result<Vec<String>, AssayError> {
    // Validate model_window is large enough for meaningful budgeting.
    if model_window <= OUTPUT_RESERVE as u64 {
        return Err(AssayError::ContextBudgetInvalid {
            message: format!(
                "model_window ({model_window}) must exceed output_reserve ({OUTPUT_RESERVE})"
            ),
        });
    }

    // Guard against u64 values that would wrap when cast to i64.
    debug_assert!(
        model_window <= i64::MAX as u64,
        "model_window exceeds i64::MAX"
    );
    let max_tokens = model_window as i64;
    let usable = max_tokens - OUTPUT_RESERVE;
    let safety = (usable as f64 * (SAFETY_MARGIN_PERCENT / 100.0)) as i64;
    let target_tokens = usable - safety;

    // Collect non-empty inputs with their token estimates in canonical order:
    // [system_prompt, spec_body, criteria_text, diff]
    let mut parts: Vec<(&str, i64)> = Vec::with_capacity(4);

    if !system_prompt.is_empty() {
        parts.push((system_prompt, estimate_tokens(system_prompt)));
    }
    if !spec_body.is_empty() {
        parts.push((spec_body, estimate_tokens(spec_body)));
    }
    if !criteria_text.is_empty() {
        parts.push((criteria_text, estimate_tokens(criteria_text)));
    }
    if !diff.is_empty() {
        parts.push((diff, estimate_tokens(diff)));
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
    // Items are inserted in canonical order (prompt, spec, criteria, diff) so that
    // ChronologicalPlacer preserves the same ordering as the passthrough path.
    let map_err = |e: cupel::CupelError| AssayError::ContextBudget { source: e };

    let mut items = Vec::with_capacity(4);

    if !system_prompt.is_empty() {
        items.push(
            ContextItemBuilder::new(system_prompt, estimate_tokens(system_prompt))
                .kind(ContextKind::new(ContextKind::SYSTEM_PROMPT).map_err(map_err)?)
                .pinned(true)
                .build()
                .map_err(map_err)?,
        );
    }

    if !spec_body.is_empty() {
        items.push(
            ContextItemBuilder::new(spec_body, estimate_tokens(spec_body))
                .kind(ContextKind::new(ContextKind::DOCUMENT).map_err(map_err)?)
                .source(ContextSource::new(ContextSource::RAG).map_err(map_err)?)
                .priority(PRIORITY_SPEC)
                .build()
                .map_err(map_err)?,
        );
    }

    if !criteria_text.is_empty() {
        items.push(
            ContextItemBuilder::new(criteria_text, estimate_tokens(criteria_text))
                .kind(ContextKind::new(ContextKind::DOCUMENT).map_err(map_err)?)
                .source(ContextSource::new(ContextSource::RAG).map_err(map_err)?)
                .pinned(true)
                .build()
                .map_err(map_err)?,
        );
    }

    if !diff.is_empty() {
        items.push(
            ContextItemBuilder::new(diff, estimate_tokens(diff))
                .kind(ContextKind::new("Diff").map_err(map_err)?)
                .source(ContextSource::new(ContextSource::TOOL).map_err(map_err)?)
                .priority(PRIORITY_DIFF)
                .build()
                .map_err(map_err)?,
        );
    }

    let pipeline = Pipeline::builder()
        .scorer(Box::new(PriorityScorer))
        .slicer(Box::new(GreedySlice))
        .placer(Box::new(ChronologicalPlacer))
        // Deduplication disabled: content sources are distinct (prompt, spec, criteria, diff)
        // and never overlap, so content-comparison would never match.
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
    use cupel::{
        ChronologicalPlacer, ContextBudget, ContextItemBuilder, ContextKind, ContextSource,
        DiagnosticTraceCollector, GreedySlice, OverflowStrategy, Pipeline, PriorityScorer,
        TraceDetailLevel,
    };
    use cupel_testing::SelectionReportAssertions;

    fn test_pipeline() -> Pipeline {
        Pipeline::builder()
            .scorer(Box::new(PriorityScorer))
            .slicer(Box::new(GreedySlice))
            .placer(Box::new(ChronologicalPlacer))
            .deduplication(false)
            .overflow_strategy(OverflowStrategy::Truncate)
            .build()
            .expect("pipeline builds")
    }

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

        // Pipeline path: result should have fewer total bytes than the input diff alone.
        let total_bytes: usize = result.iter().map(|s| s.len()).sum();
        assert!(
            total_bytes < large_diff.len(),
            "total output ({total_bytes}) should be less than input diff ({})",
            large_diff.len()
        );

        // Verify the pipeline path preserves canonical ordering.
        assert!(result.len() >= 2, "should have at least prompt + criteria");
        assert_eq!(result[0], "prompt", "system prompt should be first");
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

        // System prompt and criteria must always be present (they are pinned).
        assert!(result.len() >= 2, "must have at least pinned items");
        assert_eq!(
            result[0], "system prompt text",
            "system prompt must be first"
        );

        // Criteria must appear and must come after spec (if spec is present).
        let criteria_pos = result
            .iter()
            .position(|s| s == "criteria text")
            .expect("criteria must be included");
        assert!(criteria_pos > 0, "criteria must not be first (prompt is)");
    }

    #[test]
    fn pipeline_preserves_canonical_ordering() {
        // Use a window small enough that the pipeline path is triggered but large
        // enough that all non-empty items can fit after truncation.
        let diff = "d".repeat(100_000);
        let result =
            budget_context("prompt", "spec", "criteria", &diff, 50_000).expect("should succeed");

        // Verify ordering: prompt before spec before criteria.
        // (diff may be truncated or dropped entirely)
        let prompt_pos = result.iter().position(|s| s == "prompt");
        let spec_pos = result.iter().position(|s| s == "spec");
        let criteria_pos = result.iter().position(|s| s == "criteria");

        if let (Some(p), Some(s)) = (prompt_pos, spec_pos) {
            assert!(p < s, "prompt ({p}) must precede spec ({s})");
        }
        if let (Some(s), Some(c)) = (spec_pos, criteria_pos) {
            assert!(s < c, "spec ({s}) must precede criteria ({c})");
        }
        if let (Some(p), Some(c)) = (prompt_pos, criteria_pos) {
            assert!(p < c, "prompt ({p}) must precede criteria ({c})");
        }
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

    #[test]
    fn rejects_zero_model_window() {
        let result = budget_context("prompt", "spec", "criteria", "diff", 0);
        assert!(result.is_err(), "model_window=0 should fail");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("must exceed output_reserve"),
            "error should explain the constraint: {err}"
        );
    }

    #[test]
    fn rejects_model_window_below_output_reserve() {
        let result = budget_context("prompt", "spec", "criteria", "diff", 4_096);
        assert!(
            result.is_err(),
            "model_window=4096 should fail (equals output_reserve)"
        );
    }

    #[test]
    fn error_preserves_cupel_source() {
        // Trigger a cupel error by making pinned items exceed a tiny budget.
        // model_window = 4097 (just above OUTPUT_RESERVE) → target ≈ 0 tokens.
        let result = budget_context(&"p".repeat(10_000), "", &"c".repeat(10_000), "", 4_097);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Verify the error chain works (source() returns the CupelError).
        assert!(
            std::error::Error::source(&err).is_some(),
            "ContextBudget should preserve CupelError as source"
        );
    }

    /// budget-test-empty-system-prompt:
    /// An empty system prompt is excluded from the output, and the remaining
    /// non-empty inputs are returned in canonical order.
    #[test]
    fn empty_system_prompt_excluded_from_output() {
        let result =
            budget_context("", "spec body", "criteria", "diff", 200_000).expect("should succeed");

        assert_eq!(result.len(), 3, "empty system prompt should be excluded");
        assert_eq!(result[0], "spec body");
        assert_eq!(result[1], "criteria");
        assert_eq!(result[2], "diff");
    }

    /// Exercises `run_traced()` + `DiagnosticTraceCollector` + cupel-testing fluent assertions
    /// on a passthrough budget where all items fit within the window.
    #[test]
    fn fluent_assertions_passthrough_report() {
        let pipeline = test_pipeline();

        let items = vec![
            ContextItemBuilder::new("system prompt", 20)
                .kind(ContextKind::new(ContextKind::SYSTEM_PROMPT).unwrap())
                .pinned(true)
                .build()
                .unwrap(),
            ContextItemBuilder::new("spec body", 15)
                .kind(ContextKind::new(ContextKind::DOCUMENT).unwrap())
                .source(ContextSource::new(ContextSource::RAG).unwrap())
                .priority(80)
                .build()
                .unwrap(),
            ContextItemBuilder::new("diff content", 10)
                .kind(ContextKind::new("Diff").unwrap())
                .source(ContextSource::new(ContextSource::TOOL).unwrap())
                .priority(50)
                .build()
                .unwrap(),
        ];

        // All 3 items fit: total 45 tokens is trivially within the 190k target.
        let budget =
            ContextBudget::new(200_000, 190_000, 4_096, HashMap::new(), 5.0).expect("budget valid");

        let mut collector = DiagnosticTraceCollector::new(TraceDetailLevel::Item);
        let result = pipeline
            .run_traced(&items, &budget, &mut collector)
            .expect("pipeline succeeds");

        assert_eq!(result.len(), 3, "all items should pass through");

        let report = collector.into_report();

        report
            .should()
            .include_item_with_kind(ContextKind::new(ContextKind::SYSTEM_PROMPT).unwrap());
        report
            .should()
            .include_item_with_kind(ContextKind::new(ContextKind::DOCUMENT).unwrap());
        report
            .should()
            .include_item_with_kind(ContextKind::new("Diff").unwrap());
        report.should().have_kind_coverage_count(3);
    }

    /// Exercises the truncation path: a large diff that exceeds the token budget is excluded.
    #[test]
    fn fluent_assertions_truncation_report() {
        let pipeline = test_pipeline();

        let large_diff = "x".repeat(1_000_000);
        let diff_tokens = estimate_tokens(&large_diff);

        let items = vec![
            ContextItemBuilder::new("prompt", 10)
                .kind(ContextKind::new(ContextKind::SYSTEM_PROMPT).unwrap())
                .pinned(true)
                .build()
                .unwrap(),
            ContextItemBuilder::new(&large_diff, diff_tokens)
                .kind(ContextKind::new("Diff").unwrap())
                .source(ContextSource::new(ContextSource::TOOL).unwrap())
                .priority(50)
                .build()
                .unwrap(),
        ];

        // Small budget: 50k window → ~43k target. The diff (~1 MB, ≈270k tokens at
        // 3.7 bytes/token) vastly exceeds the budget, forcing truncation.
        let budget =
            ContextBudget::new(50_000, 43_000, 4_096, HashMap::new(), 5.0).expect("budget valid");

        let mut collector = DiagnosticTraceCollector::new(TraceDetailLevel::Item);
        let result = pipeline
            .run_traced(&items, &budget, &mut collector)
            .expect("pipeline succeeds");

        // The pinned prompt is always included; the oversized diff is the only exclusion
        // candidate. Asserting ≥1 rather than ==1 because truncation may split fragments.
        assert_eq!(
            result.len(),
            1,
            "only the pinned prompt should survive the budget"
        );
        let report = collector.into_report();
        report.should().have_at_least_n_exclusions(1);
    }
}
